use log::{info, warn};
use serde_derive::{Deserialize, Serialize};

use crate::creds::seedance_api_key::SeedanceApiKey;
use crate::creds::seedance_base_url::SeedanceBaseUrl;
use crate::creds::seedance_provider::SeedanceProvider;
use crate::error::seedance_api_error::SeedanceApiError;
use crate::error::seedance_client_error::SeedanceClientError;
use crate::error::seedance_error::SeedanceError;

/// The `content[].role` values that mark frame images. Both providers use these
/// exact spellings; an image with no role is read as a first frame, but keyframe
/// requests are clearer with it spelled out.
const FIRST_FRAME_ROLE: &str = "first_frame";
const LAST_FRAME_ROLE: &str = "last_frame";

/// Tunable generation parameters, forwarded verbatim. `None` fields are omitted
/// so the API applies its own defaults.
#[derive(Clone, Debug, Default, Serialize)]
pub struct VideoParameters {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub resolution: Option<String>,

  #[serde(skip_serializing_if = "Option::is_none")]
  pub ratio: Option<String>,

  #[serde(skip_serializing_if = "Option::is_none")]
  pub duration: Option<i32>,

  #[serde(skip_serializing_if = "Option::is_none")]
  pub seed: Option<i64>,

  #[serde(skip_serializing_if = "Option::is_none")]
  pub generate_audio: Option<bool>,

  #[serde(skip_serializing_if = "Option::is_none")]
  pub output_format: Option<String>,
}

/// Arguments for [`submit_video_generation`].
///
/// The frames are public URLs — the API fetches them itself. Pass `None` for
/// text-to-video; keyframe generation uses `start_frame_url` alone, or both
/// URLs for a first-and-last-frame clip.
#[derive(Clone, Debug)]
pub struct VideoGenerationArgs<'a> {
  pub base_url: &'a SeedanceBaseUrl,
  pub api_key: &'a SeedanceApiKey,
  pub model: &'a str,
  pub prompt: &'a str,
  pub maybe_start_frame_url: Option<&'a str>,
  pub maybe_end_frame_url: Option<&'a str>,
  pub parameters: VideoParameters,
}

/// The task id returned after a successful submit.
#[derive(Clone, Debug, Serialize)]
pub struct SubmittedVideoTask {
  pub task_id: String,
}

/// ARK task status, collapsed to the states a poller acts on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SeedanceTaskStatus {
  Pending,
  Running,
  Succeeded { video_url: String },
  Failed { message: Option<String> },
  Expired,
  Unknown,
}

/// Submit a video generation task and return its task id.
///
/// The body shape and the returned task id depend on the provider selected on
/// `base_url`. The API enqueues the job; poll the returned id with
/// [`query_video_task`].
pub async fn submit_video_generation(args: VideoGenerationArgs<'_>) -> Result<SubmittedVideoTask, SeedanceError> {
  let url = args.base_url.submit_url();

  let request_body = serialize_submit_body(&args)?;

  info!("Submitting Seedance video generation: provider={:?} url={}", args.base_url.provider(), url);
  info!("Seedance submit args: {:?}", args);
  info!("Seedance submit request body: {}", request_body);

  let client = reqwest::Client::builder().build().map_err(SeedanceClientError::ReqwestClientError)?;

  let response = client.post(&url).bearer_auth(args.api_key.as_str()).header("Content-Type", "application/json").body(request_body).send().await.map_err(SeedanceApiError::ReqwestError)?;

  let status = response.status();
  let response_body = response.text().await.map_err(SeedanceApiError::ReqwestError)?;

  if !status.is_success() {
    return Err(SeedanceApiError::from_status_and_body(status, response_body).into());
  }

  Ok(SubmittedVideoTask { task_id: parse_submitted_task_id(args.base_url.provider(), &response_body)? })
}

/// Query a submitted task's status.
///
/// `GET {base}/contents/generations/tasks/{task_id}` for Volcano ARK, or
/// `GET {base}/tasks/status?task_id={task_id}` for ModelVerse (whose base URL
/// already carries its `/v1` prefix). Terminal states carry the generated video
/// URL on success.
pub async fn query_video_task(base_url: &SeedanceBaseUrl, api_key: &SeedanceApiKey, task_id: &str) -> Result<SeedanceTaskStatus, SeedanceError> {
  let url = base_url.task_url(task_id);

  let client = reqwest::Client::builder().build().map_err(SeedanceClientError::ReqwestClientError)?;

  let response = client.get(&url).bearer_auth(api_key.as_str()).send().await.map_err(SeedanceApiError::ReqwestError)?;

  let status = response.status();
  let body = response.text().await.map_err(SeedanceApiError::ReqwestError)?;

  if !status.is_success() {
    return Err(SeedanceApiError::from_status_and_body(status, body).into());
  }

  parse_task_status(base_url.provider(), &body)
}

// ── Per-provider request/response shaping ──

/// Serialize the submit body in the shape the selected provider expects.
///
/// Both providers take the same `content` array — the prompt plus the frame
/// images, each tagged with the `role` that says where it belongs — but hold
/// the tunable parameters at different levels: ARK inlines them at the top
/// level, ModelVerse nests them under `parameters`.
fn serialize_submit_body(args: &VideoGenerationArgs<'_>) -> Result<String, SeedanceError> {
  match args.base_url.provider() {
    SeedanceProvider::VolcanoArk => {
      let body = ArkSubmitRequestBody { model: args.model, content: build_content(args), resolution: args.parameters.resolution.clone(), ratio: args.parameters.ratio.clone(), duration: args.parameters.duration, seed: args.parameters.seed, generate_audio: args.parameters.generate_audio, output_format: args.parameters.output_format.clone() };
      Ok(serde_json::to_string(&body).map_err(SeedanceClientError::RequestSerializationError)?)
    },
    SeedanceProvider::ModelVerse => {
      let body = ModelVerseSubmitRequestBody { model: args.model, input: ModelVerseSubmitInput { content: build_content(args) }, parameters: &args.parameters };
      Ok(serde_json::to_string(&body).map_err(SeedanceClientError::RequestSerializationError)?)
    },
  }
}

/// Assemble the prompt and any frame images into the submit `content` array.
/// The prompt leads; frames follow in first-then-last order. A last frame
/// without a first is not a shape the API accepts, so it is dropped.
fn build_content<'a>(args: &VideoGenerationArgs<'a>) -> Vec<SubmitContentItem<'a>> {
  let mut content = vec![SubmitContentItem::text(args.prompt)];

  match (args.maybe_start_frame_url, args.maybe_end_frame_url) {
    (Some(start_frame_url), maybe_end_frame_url) => {
      content.push(SubmitContentItem::image(start_frame_url, FIRST_FRAME_ROLE));

      if let Some(end_frame_url) = maybe_end_frame_url {
        content.push(SubmitContentItem::image(end_frame_url, LAST_FRAME_ROLE));
      }
    },
    (None, maybe_end_frame_url) => {
      if maybe_end_frame_url.is_some() {
        warn!("Dropping the Seedance end frame: an end frame without a start frame is not a supported request shape");
      }
    },
  }

  content
}

/// Pull the task id out of a submit response.
fn parse_submitted_task_id(provider: SeedanceProvider, response_body: &str) -> Result<String, SeedanceError> {
  match provider {
    SeedanceProvider::VolcanoArk => {
      let parsed: ArkSubmitResponseBody = serde_json::from_str(response_body).map_err(|err| SeedanceApiError::SerdeResponseParseErrorWithBody(err, response_body.to_string()))?;
      Ok(parsed.id)
    },
    SeedanceProvider::ModelVerse => {
      let parsed: ModelVerseSubmitResponseBody = serde_json::from_str(response_body).map_err(|err| SeedanceApiError::SerdeResponseParseErrorWithBody(err, response_body.to_string()))?;
      Ok(parsed.output.task_id)
    },
  }
}

/// Map a query response onto the provider-independent [`SeedanceTaskStatus`].
fn parse_task_status(provider: SeedanceProvider, response_body: &str) -> Result<SeedanceTaskStatus, SeedanceError> {
  match provider {
    SeedanceProvider::VolcanoArk => {
      let parsed: ArkQueryResponseBody = serde_json::from_str(response_body).map_err(|err| SeedanceApiError::SerdeResponseParseErrorWithBody(err, response_body.to_string()))?;

      Ok(match parsed.status.as_str() {
        "queued" => SeedanceTaskStatus::Pending,
        "running" => SeedanceTaskStatus::Running,
        "succeeded" => SeedanceTaskStatus::Succeeded { video_url: parsed.content.and_then(|c| c.video_url).unwrap_or_default() },
        "failed" => SeedanceTaskStatus::Failed { message: parsed.error.as_ref().and_then(|e| e.get("message")).and_then(serde_json::Value::as_str).map(str::to_string) },
        "expired" => SeedanceTaskStatus::Expired,
        _ => SeedanceTaskStatus::Unknown,
      })
    },
    SeedanceProvider::ModelVerse => {
      let parsed: ModelVerseQueryResponseBody = serde_json::from_str(response_body).map_err(|err| SeedanceApiError::SerdeResponseParseErrorWithBody(err, response_body.to_string()))?;

      Ok(match parsed.output.task_status.as_str() {
        "Pending" => SeedanceTaskStatus::Pending,
        "Running" => SeedanceTaskStatus::Running,
        "Success" => SeedanceTaskStatus::Succeeded { video_url: parsed.output.urls.into_iter().next().unwrap_or_default() },
        "Failure" => SeedanceTaskStatus::Failed { message: parsed.output.error_message },
        _ => SeedanceTaskStatus::Unknown,
      })
    },
  }
}

// ── Wire-format DTOs (private) ──

#[derive(Serialize)]
struct ArkSubmitRequestBody<'a> {
  model: &'a str,
  content: Vec<SubmitContentItem<'a>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  resolution: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  ratio: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  duration: Option<i32>,
  #[serde(skip_serializing_if = "Option::is_none")]
  seed: Option<i64>,
  #[serde(skip_serializing_if = "Option::is_none")]
  generate_audio: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  output_format: Option<String>,
}

#[derive(Serialize)]
struct ModelVerseSubmitRequestBody<'a> {
  model: &'a str,
  input: ModelVerseSubmitInput<'a>,
  parameters: &'a VideoParameters,
}

#[derive(Serialize)]
struct ModelVerseSubmitInput<'a> {
  content: Vec<SubmitContentItem<'a>>,
}

/// One entry of the submit `content` array: either the prompt, or a frame image
/// tagged with its `role`. The unused fields are omitted, so a text entry
/// carries only `text` and an image entry only `image_url` and `role`.
#[derive(Serialize)]
struct SubmitContentItem<'a> {
  #[serde(rename = "type")]
  r#type: &'a str,

  #[serde(skip_serializing_if = "Option::is_none")]
  text: Option<&'a str>,

  #[serde(skip_serializing_if = "Option::is_none")]
  image_url: Option<SubmitImageUrl<'a>>,

  #[serde(skip_serializing_if = "Option::is_none")]
  role: Option<&'a str>,
}

#[derive(Serialize)]
struct SubmitImageUrl<'a> {
  url: &'a str,
}

impl<'a> SubmitContentItem<'a> {
  fn text(text: &'a str) -> Self {
    Self { r#type: "text", text: Some(text), image_url: None, role: None }
  }

  fn image(url: &'a str, role: &'a str) -> Self {
    Self { r#type: "image_url", text: None, image_url: Some(SubmitImageUrl { url }), role: Some(role) }
  }
}

#[derive(Deserialize)]
struct ArkSubmitResponseBody {
  id: String,
}

#[derive(Deserialize)]
struct ModelVerseSubmitResponseBody {
  output: ModelVerseSubmitOutput,
}

#[derive(Deserialize)]
struct ModelVerseSubmitOutput {
  task_id: String,
}

#[derive(Deserialize)]
struct ArkQueryResponseBody {
  status: String,
  #[serde(default)]
  content: Option<QueryContent>,
  #[serde(default)]
  error: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct QueryContent {
  #[serde(default)]
  video_url: Option<String>,
}

#[derive(Deserialize)]
struct ModelVerseQueryResponseBody {
  output: ModelVerseQueryOutput,
}

#[derive(Deserialize)]
struct ModelVerseQueryOutput {
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
  fn submit_body_serializes_to_ark_shape() {
    let body = ArkSubmitRequestBody { model: "doubao-seedance-2-0-260128", content: vec![SubmitContentItem::text("a cat")], resolution: Some("720p".into()), ratio: Some("16:9".into()), duration: Some(5), seed: Some(11), generate_audio: Some(true), output_format: Some("mp4".into()) };

    let value = serde_json::to_value(&body).unwrap();
    assert_eq!(value["model"], "doubao-seedance-2-0-260128");
    assert_eq!(value["content"][0]["type"], "text");
    assert_eq!(value["content"][0]["text"], "a cat");
    assert_eq!(value["resolution"], "720p");
    assert_eq!(value["ratio"], "16:9");
    assert_eq!(value["duration"], 5);
    assert_eq!(value["generate_audio"], true);
    assert_eq!(value["output_format"], "mp4");
  }

  #[test]
  fn submit_body_omits_unset_parameters() {
    let body = ArkSubmitRequestBody { model: "doubao-seedance-2-0-260128", content: vec![SubmitContentItem::text("a cat")], resolution: None, ratio: None, duration: None, seed: None, generate_audio: None, output_format: None };

    let value = serde_json::to_value(&body).unwrap();
    assert!(value.get("resolution").is_none());
    assert!(value.get("ratio").is_none());
    assert!(value.get("duration").is_none());
    assert!(value.get("seed").is_none());
    assert!(value.get("generate_audio").is_none());
    assert!(value.get("output_format").is_none());
  }

  mod content_tests {
    use super::*;

    #[test]
    fn text_to_video_sends_the_prompt_alone() {
      let value = content_value(None, None);

      assert_eq!(value, serde_json::json!([{ "type": "text", "text": "a cat" }]));
    }

    #[test]
    fn keyframes_are_tagged_with_their_roles() {
      let value = content_value(Some("https://example.com/first.png"), Some("https://example.com/last.png"));

      assert_eq!(value[1]["type"], "image_url");
      assert_eq!(value[1]["image_url"]["url"], "https://example.com/first.png");
      assert_eq!(value[1]["role"], "first_frame");
      assert_eq!(value[2]["image_url"]["url"], "https://example.com/last.png");
      assert_eq!(value[2]["role"], "last_frame");
    }

    #[test]
    fn a_lone_end_frame_is_dropped() {
      let value = content_value(None, Some("https://example.com/last.png"));

      assert_eq!(value.as_array().unwrap().len(), 1);
      assert_eq!(value[0]["type"], "text");
    }

    /// Serialize the `content` array for a submit carrying the given keyframes.
    fn content_value(maybe_start_frame_url: Option<&str>, maybe_end_frame_url: Option<&str>) -> serde_json::Value {
      let base_url = SeedanceBaseUrl::new("https://api.modelverse.cn/v1", SeedanceProvider::ModelVerse);
      let api_key = SeedanceApiKey::new("secret");

      let args = VideoGenerationArgs { base_url: &base_url, api_key: &api_key, model: "doubao-seedance-2-0-260128", prompt: "a cat", maybe_start_frame_url, maybe_end_frame_url, parameters: VideoParameters::default() };

      serde_json::to_value(build_content(&args)).unwrap()
    }
  }

  mod modelverse_tests {
    use super::*;

    #[test]
    fn submit_body_wraps_content_and_parameters() {
      let body = ModelVerseSubmitRequestBody { model: "doubao-seedance-1-5-pro-251215", input: ModelVerseSubmitInput { content: vec![SubmitContentItem::text("a cat")] }, parameters: &VideoParameters { resolution: Some("720p".into()), ratio: Some("16:9".into()), duration: Some(5), seed: None, generate_audio: Some(true), output_format: None } };

      let value = serde_json::to_value(&body).unwrap();
      assert_eq!(value["model"], "doubao-seedance-1-5-pro-251215");
      assert_eq!(value["input"]["content"][0]["type"], "text");
      assert_eq!(value["input"]["content"][0]["text"], "a cat");
      assert_eq!(value["parameters"]["resolution"], "720p");
      assert_eq!(value["parameters"]["duration"], 5);
      assert_eq!(value["parameters"]["generate_audio"], true);
      assert!(value["parameters"].get("seed").is_none());
      // The parameters live under `parameters`, not at the top level.
      assert!(value.get("resolution").is_none());
    }

    #[test]
    fn submit_response_carries_the_task_id_under_output() {
      let task_id = parse_submitted_task_id(SeedanceProvider::ModelVerse, r#"{"output":{"task_id":"task_123"}}"#).unwrap();

      assert_eq!(task_id, "task_123");
    }

    #[test]
    fn query_response_maps_unified_statuses() {
      let succeeded = parse_task_status(SeedanceProvider::ModelVerse, r#"{"output":{"task_id":"task_123","task_status":"Success","urls":["https://example.com/a.mp4"]}}"#).unwrap();
      assert_eq!(succeeded, SeedanceTaskStatus::Succeeded { video_url: "https://example.com/a.mp4".to_string() });

      let failed = parse_task_status(SeedanceProvider::ModelVerse, r#"{"output":{"task_id":"task_123","task_status":"Failure","error_message":"boom"}}"#).unwrap();
      assert_eq!(failed, SeedanceTaskStatus::Failed { message: Some("boom".to_string()) });

      let pending = parse_task_status(SeedanceProvider::ModelVerse, r#"{"output":{"task_status":"Pending"}}"#).unwrap();
      assert_eq!(pending, SeedanceTaskStatus::Pending);

      let running = parse_task_status(SeedanceProvider::ModelVerse, r#"{"output":{"task_status":"Running"}}"#).unwrap();
      assert_eq!(running, SeedanceTaskStatus::Running);

      let unknown = parse_task_status(SeedanceProvider::ModelVerse, r#"{"output":{"task_status":"Weird"}}"#).unwrap();
      assert_eq!(unknown, SeedanceTaskStatus::Unknown);
    }
  }

  mod ark_tests {
    use super::*;

    #[test]
    fn submit_response_carries_the_task_id_at_the_top_level() {
      let task_id = parse_submitted_task_id(SeedanceProvider::VolcanoArk, r#"{"id":"task_123"}"#).unwrap();

      assert_eq!(task_id, "task_123");
    }

    #[test]
    fn query_response_maps_ark_statuses() {
      let succeeded = parse_task_status(SeedanceProvider::VolcanoArk, r#"{"status":"succeeded","content":{"video_url":"https://example.com/a.mp4"}}"#).unwrap();
      assert_eq!(succeeded, SeedanceTaskStatus::Succeeded { video_url: "https://example.com/a.mp4".to_string() });

      let queued = parse_task_status(SeedanceProvider::VolcanoArk, r#"{"status":"queued"}"#).unwrap();
      assert_eq!(queued, SeedanceTaskStatus::Pending);

      let failed = parse_task_status(SeedanceProvider::VolcanoArk, r#"{"status":"failed","error":{"message":"boom"}}"#).unwrap();
      assert_eq!(failed, SeedanceTaskStatus::Failed { message: Some("boom".to_string()) });

      let expired = parse_task_status(SeedanceProvider::VolcanoArk, r#"{"status":"expired"}"#).unwrap();
      assert_eq!(expired, SeedanceTaskStatus::Expired);
    }
  }
}
