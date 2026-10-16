use log::info;
use serde_derive::{Deserialize, Serialize};

use crate::creds::wan_api_key::WanApiKey;
use crate::creds::wan_base_url::WanBaseUrl;
use crate::creds::wan_provider::WanProvider;
use crate::error::wan_api_error::WanApiError;
use crate::error::wan_client_error::WanClientError;
use crate::error::wan_error::WanError;

/// Map the desktop omni model id (`wan_3p0` / `wan_3p0_prime`) to the DashScope
/// model id the gateway expects (`wan3.0-video` / `wan3.0-video-prime`).
pub fn dashscope_model_id(omni_model: &str) -> Option<&'static str> {
  match omni_model {
    "wan_3p0" => Some("wan3.0-video"),
    "wan_3p0_prime" => Some("wan3.0-video-prime"),
    _ => None,
  }
}

/// Tunable generation parameters, forwarded to the gateway verbatim. `None`
/// fields are omitted so the gateway applies its own defaults.
#[derive(Clone, Debug, Default, Serialize)]
pub struct VideoParameters {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub resolution: Option<String>,

  #[serde(skip_serializing_if = "Option::is_none")]
  pub ratio: Option<String>,

  #[serde(skip_serializing_if = "Option::is_none")]
  pub duration: Option<i32>,

  #[serde(skip_serializing_if = "Option::is_none")]
  pub audio: Option<bool>,

  #[serde(skip_serializing_if = "Option::is_none")]
  pub seed: Option<i64>,
}

/// Arguments for [`submit_video_synthesis`].
#[derive(Clone, Debug)]
pub struct VideoSynthesisArgs<'a> {
  pub base_url: &'a WanBaseUrl,
  pub api_key: &'a WanApiKey,
  pub model: &'a str,
  pub prompt: &'a str,
  pub parameters: VideoParameters,
}

/// The task id returned after a successful submit.
#[derive(Clone, Debug, Serialize)]
pub struct SubmittedVideoTask {
  pub task_id: String,
}

/// DashScope task status, collapsed to the states a poller acts on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WanTaskStatus {
  Pending,
  Running,
  Succeeded { video_url: String },
  Failed { code: Option<String>, message: Option<String> },
  Canceled,
  Unknown,
}

/// Submit a text-to-video synthesis task and return its task id.
///
/// The body shape and the extra headers depend on the provider selected on
/// `base_url`. The gateway enqueues the job; poll the returned id with
/// [`query_video_task`].
pub async fn submit_video_synthesis(args: VideoSynthesisArgs<'_>) -> Result<SubmittedVideoTask, WanError> {
  let url = args.base_url.video_synthesis_url();

  let request_body = serialize_submit_body(&args)?;

  info!("Submitting Wan video synthesis: provider={:?} url={}", args.base_url.provider(), url);
  info!("Wan submit args: {:?}", args);
  info!("Wan submit request body: {}", request_body);

  let client = reqwest::Client::builder().build().map_err(WanClientError::ReqwestClientError)?;

  let mut request = client.post(&url).bearer_auth(args.api_key.as_str()).header("Content-Type", "application/json").body(request_body);

  // Only DashScope-compatible gateways take the async-video flag.
  if args.base_url.provider() == WanProvider::DashScope {
    request = request.header("X-DashScope-Async", "enable");
  }

  let response = request.send().await.map_err(WanApiError::ReqwestError)?;

  let status = response.status();
  let response_body = response.text().await.map_err(WanApiError::ReqwestError)?;

  if !status.is_success() {
    return Err(WanApiError::from_status_and_body(status, response_body).into());
  }

  let parsed: SubmitResponseBody = serde_json::from_str(&response_body).map_err(|err| WanApiError::SerdeResponseParseErrorWithBody(err, response_body.clone()))?;

  Ok(SubmittedVideoTask { task_id: parsed.output.task_id })
}

/// Query a submitted task's status.
///
/// `GET {base}/api/v1/tasks/{task_id}` for DashScope, or
/// `GET {base}/v1/tasks/status?task_id={task_id}` for ModelVerse. Terminal
/// states carry either a video URL (success) or a failure reason; an empty
/// video URL on success is left to the caller to treat as a data problem.
pub async fn query_video_task(base_url: &WanBaseUrl, api_key: &WanApiKey, task_id: &str) -> Result<WanTaskStatus, WanError> {
  let url = base_url.task_url(task_id);

  let client = reqwest::Client::builder().build().map_err(WanClientError::ReqwestClientError)?;

  let response = client.get(&url).bearer_auth(api_key.as_str()).send().await.map_err(WanApiError::ReqwestError)?;

  let status = response.status();
  let body = response.text().await.map_err(WanApiError::ReqwestError)?;

  if !status.is_success() {
    return Err(WanApiError::from_status_and_body(status, body).into());
  }

  parse_task_status(base_url.provider(), &body)
}

// ── Per-provider request/response shaping ──

/// Serialize the submit body in the shape the selected provider expects.
fn serialize_submit_body(args: &VideoSynthesisArgs<'_>) -> Result<String, WanError> {
  match args.base_url.provider() {
    WanProvider::DashScope => {
      let body = DashScopeSubmitRequestBody { model: args.model, input: DashScopeSubmitInput { prompt: args.prompt }, parameters: &args.parameters };
      Ok(serde_json::to_string(&body).map_err(WanClientError::RequestSerializationError)?)
    },
    WanProvider::ModelVerse => {
      let body = ModelVerseSubmitRequestBody { model: args.model, input: ModelVerseSubmitInput { content: vec![ModelVerseSubmitContent { r#type: "text", text: args.prompt }] }, parameters: &args.parameters };
      Ok(serde_json::to_string(&body).map_err(WanClientError::RequestSerializationError)?)
    },
  }
}

/// Map a query response onto the provider-independent [`WanTaskStatus`].
///
/// Both providers use the same `output.task_status` vocabulary but different
/// casing and different result/error fields, so the status words are compared
/// case-insensitively and each provider gets its own body shape.
fn parse_task_status(provider: WanProvider, response_body: &str) -> Result<WanTaskStatus, WanError> {
  match provider {
    WanProvider::DashScope => {
      let parsed: DashScopeQueryResponseBody = serde_json::from_str(response_body).map_err(|err| WanApiError::SerdeResponseParseErrorWithBody(err, response_body.to_string()))?;

      Ok(match parsed.output.task_status.to_lowercase().as_str() {
        "pending" => WanTaskStatus::Pending,
        "running" => WanTaskStatus::Running,
        "succeeded" | "success" => WanTaskStatus::Succeeded { video_url: parsed.output.video_url.unwrap_or_default() },
        "failed" | "failure" => WanTaskStatus::Failed { code: parsed.output.code, message: parsed.output.message },
        "canceled" | "cancelled" => WanTaskStatus::Canceled,
        _ => WanTaskStatus::Unknown,
      })
    },
    WanProvider::ModelVerse => {
      let parsed: ModelVerseQueryResponseBody = serde_json::from_str(response_body).map_err(|err| WanApiError::SerdeResponseParseErrorWithBody(err, response_body.to_string()))?;

      Ok(match parsed.output.task_status.to_lowercase().as_str() {
        "pending" => WanTaskStatus::Pending,
        "running" => WanTaskStatus::Running,
        "succeeded" | "success" => WanTaskStatus::Succeeded { video_url: parsed.output.urls.into_iter().next().unwrap_or_default() },
        "failed" | "failure" => WanTaskStatus::Failed { code: None, message: parsed.output.error_message },
        "canceled" | "cancelled" => WanTaskStatus::Canceled,
        _ => WanTaskStatus::Unknown,
      })
    },
  }
}

// ── Wire-format DTOs (private) ──

#[derive(Serialize)]
struct DashScopeSubmitRequestBody<'a> {
  model: &'a str,
  input: DashScopeSubmitInput<'a>,
  parameters: &'a VideoParameters,
}

#[derive(Serialize)]
struct DashScopeSubmitInput<'a> {
  prompt: &'a str,
}

#[derive(Serialize)]
struct ModelVerseSubmitRequestBody<'a> {
  model: &'a str,
  input: ModelVerseSubmitInput<'a>,
  parameters: &'a VideoParameters,
}

#[derive(Serialize)]
struct ModelVerseSubmitInput<'a> {
  content: Vec<ModelVerseSubmitContent<'a>>,
}

#[derive(Serialize)]
struct ModelVerseSubmitContent<'a> {
  #[serde(rename = "type")]
  r#type: &'a str,
  text: &'a str,
}

#[derive(Deserialize)]
struct SubmitResponseBody {
  output: SubmitOutputBody,
  #[allow(dead_code)]
  request_id: Option<String>,
}

#[derive(Deserialize)]
struct SubmitOutputBody {
  task_id: String,
  #[allow(dead_code)]
  task_status: Option<String>,
  #[allow(dead_code)]
  code: Option<String>,
  #[allow(dead_code)]
  message: Option<String>,
}

#[derive(Deserialize)]
struct DashScopeQueryResponseBody {
  output: DashScopeQueryOutputBody,
  #[allow(dead_code)]
  request_id: Option<String>,
}

#[derive(Deserialize)]
struct DashScopeQueryOutputBody {
  task_status: String,
  video_url: Option<String>,
  code: Option<String>,
  message: Option<String>,
}

#[derive(Deserialize)]
struct ModelVerseQueryResponseBody {
  output: ModelVerseQueryOutputBody,
}

#[derive(Deserialize)]
struct ModelVerseQueryOutputBody {
  task_status: String,
  #[serde(default)]
  urls: Vec<String>,
  #[serde(default)]
  error_message: Option<String>,
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn maps_omni_ids_to_dashscope_ids() {
    assert_eq!(dashscope_model_id("wan_3p0"), Some("wan3.0-video"));
    assert_eq!(dashscope_model_id("wan_3p0_prime"), Some("wan3.0-video-prime"));
    assert_eq!(dashscope_model_id("seedance_2p0"), None);
  }

  #[test]
  fn submit_body_serializes_to_dashscope_shape() {
    let body = DashScopeSubmitRequestBody { model: "wan3.0-video", input: DashScopeSubmitInput { prompt: "a cat" }, parameters: &VideoParameters { resolution: Some("720P".into()), ratio: Some("16:9".into()), duration: Some(5), audio: Some(true), seed: None } };

    let value = serde_json::to_value(&body).unwrap();
    assert_eq!(value["model"], "wan3.0-video");
    assert_eq!(value["input"]["prompt"], "a cat");
    assert_eq!(value["parameters"]["resolution"], "720P");
    assert_eq!(value["parameters"]["ratio"], "16:9");
    assert_eq!(value["parameters"]["duration"], 5);
    assert!(value["parameters"].get("seed").is_none());
  }

  mod modelverse_tests {
    use super::*;

    #[test]
    fn submit_body_wraps_the_prompt_in_input_content() {
      let body = ModelVerseSubmitRequestBody { model: "wan3.0-video", input: ModelVerseSubmitInput { content: vec![ModelVerseSubmitContent { r#type: "text", text: "a cat" }] }, parameters: &VideoParameters { resolution: Some("720p".into()), ratio: Some("16:9".into()), duration: Some(5), audio: None, seed: None } };

      let value = serde_json::to_value(&body).unwrap();
      assert_eq!(value["model"], "wan3.0-video");
      assert_eq!(value["input"]["content"][0]["type"], "text");
      assert_eq!(value["input"]["content"][0]["text"], "a cat");
      assert_eq!(value["parameters"]["resolution"], "720p");
      assert_eq!(value["parameters"]["duration"], 5);
      assert!(value["parameters"].get("audio").is_none());
      // The prompt lives under `input.content`, not `input.prompt`.
      assert!(value["input"].get("prompt").is_none());
    }

    #[test]
    fn query_response_maps_unified_statuses() {
      let succeeded = parse_task_status(WanProvider::ModelVerse, r#"{"output":{"task_id":"task_123","task_status":"Success","urls":["https://example.com/a.mp4"]}}"#).unwrap();
      assert_eq!(succeeded, WanTaskStatus::Succeeded { video_url: "https://example.com/a.mp4".to_string() });

      let failed = parse_task_status(WanProvider::ModelVerse, r#"{"output":{"task_id":"task_123","task_status":"Failure","error_message":"boom"}}"#).unwrap();
      assert_eq!(failed, WanTaskStatus::Failed { code: None, message: Some("boom".to_string()) });

      let pending = parse_task_status(WanProvider::ModelVerse, r#"{"output":{"task_status":"Pending"}}"#).unwrap();
      assert_eq!(pending, WanTaskStatus::Pending);

      let running = parse_task_status(WanProvider::ModelVerse, r#"{"output":{"task_status":"Running"}}"#).unwrap();
      assert_eq!(running, WanTaskStatus::Running);
    }

    #[test]
    fn query_response_tolerates_a_missing_url_list() {
      let status = parse_task_status(WanProvider::ModelVerse, r#"{"output":{"task_status":"Success"}}"#).unwrap();

      assert_eq!(status, WanTaskStatus::Succeeded { video_url: String::new() });
    }
  }

  mod dashscope_tests {
    use super::*;

    #[test]
    fn query_response_maps_dashscope_statuses() {
      let succeeded = parse_task_status(WanProvider::DashScope, r#"{"output":{"task_id":"task_123","task_status":"SUCCEEDED","video_url":"https://example.com/a.mp4"}}"#).unwrap();
      assert_eq!(succeeded, WanTaskStatus::Succeeded { video_url: "https://example.com/a.mp4".to_string() });

      let failed = parse_task_status(WanProvider::DashScope, r#"{"output":{"task_status":"FAILED","code":"InvalidParameter","message":"boom"}}"#).unwrap();
      assert_eq!(failed, WanTaskStatus::Failed { code: Some("InvalidParameter".to_string()), message: Some("boom".to_string()) });

      let canceled = parse_task_status(WanProvider::DashScope, r#"{"output":{"task_status":"CANCELED"}}"#).unwrap();
      assert_eq!(canceled, WanTaskStatus::Canceled);

      let unknown = parse_task_status(WanProvider::DashScope, r#"{"output":{"task_status":"WHAT"}}"#).unwrap();
      assert_eq!(unknown, WanTaskStatus::Unknown);
    }
  }
}
