use enums::common::generation_provider::GenerationProvider;
use enums::tauri::tasks::task_type::TaskType;
use log::{error, info};
use serde_json::Value;
use tauri::{AppHandle, Manager};
use wan_client::api::requests::video_generation::{dashscope_model_id, submit_video_synthesis, VideoParameters, VideoSynthesisArgs};
use wan_client::creds::wan_api_key::WanApiKey;
use wan_client::creds::wan_base_url::WanBaseUrl;

use crate::core::commands::enqueue::common::notify_frontend_of_errors::notify_frontend_of_errors;
use crate::core::commands::enqueue::generate_error::{BadInputReason, GenerateError};
use crate::core::commands::enqueue::task_enqueue_success::TaskEnqueueSuccess;
use crate::core::commands::generate::generate_video::common::video_option_wire_values::{wire_ratio, wire_resolution};
use crate::core::commands::generate::generate_video::request::{TauriGenerateVideoErrorType, TauriGenerateVideoResponse};
use crate::core::commands::generate::omni::request::OmniRequest;
use crate::core::commands::response::failure_response_wrapper::{CommandErrorResponseWrapper, CommandErrorStatus};
use crate::core::commands::response::shorthand::Response;
use crate::core::events::basic_sendable_event_trait::BasicSendableEvent;
use crate::core::events::functional_events::credits_balance_changed_event::CreditsBalanceChangedEvent;
use crate::core::events::generation_events::common::GenerationModel;
use crate::core::events::generation_events::generation_enqueue_success_event::GenerationEnqueueSuccessEvent;
use crate::core::providers::credentials::payload::provider_credential_payload::ProviderCredentialPayload;
use crate::core::providers::credentials::provider_credential_key::ProviderCredentialKey;
use crate::core::providers::credentials::provider_credential_loading_cache::ProviderCredentialLoadingCache;
use crate::core::providers::selected_upstream_providers::WAN_PROVIDER;
use crate::core::state::task_database::TaskDatabase;

/// Submit a Wan text-to-video task against the user's own gateway and return
/// the enqueued task. The gateway task id becomes the provider job id, polled
/// by the third-party polling thread.
pub async fn handle_wan_video(request: &OmniRequest, credential_cache: &ProviderCredentialLoadingCache) -> Result<TaskEnqueueSuccess, GenerateError> {
  let model = request.model().ok_or_else(GenerateError::no_model_specified)?;
  let dashscope_model = dashscope_model_id(model).ok_or_else(|| GenerateError::NotYetImplemented(format!("Wan model {:?} is not supported", model)))?;

  let base_url = load_credential_string(credential_cache, ProviderCredentialKey::WanBaseUrl).ok_or_else(GenerateError::needs_wan_api_key)?;
  let api_key = load_credential_string(credential_cache, ProviderCredentialKey::WanApiKey).ok_or_else(GenerateError::needs_wan_api_key)?;

  let prompt = request.fields.get("prompt").and_then(Value::as_str).unwrap_or_default();

  let base_url = WanBaseUrl::new(base_url, WAN_PROVIDER);
  let api_key = WanApiKey::new(api_key);

  info!("Submitting Wan video generation via gateway: model={:?}", dashscope_model);

  let submitted = submit_video_synthesis(VideoSynthesisArgs { base_url: &base_url, api_key: &api_key, model: dashscope_model, prompt, parameters: build_parameters(&request.fields) }).await.map_err(|err| {
    error!("Wan video submission failed: {:?}", err);
    GenerateError::from(err)
  })?;

  // Record the endpoints this task actually used, so a failure can be traced
  // back to the gateway that was called.
  let request_url = base_url.video_synthesis_url();
  let queue_status_url = base_url.task_url(&submitted.task_id);

  Ok(TaskEnqueueSuccess { task_type: TaskType::VideoGeneration, model: Some(GenerationModel::Unknown(model.to_string())), provider: GenerationProvider::Wan, provider_job_id: Some(submitted.task_id), maybe_request_url: Some(request_url), maybe_queue_status_url: Some(queue_status_url), maybe_queue_response_url: None, maybe_prompt_token: None })
}

/// Command-level entry point for the BYOK path: submit, persist to the task
/// database, and emit the usual enqueue/credits events. Returns the same
/// response shape as the legacy native video path so callers can reuse
/// `adapt_legacy_response`.
pub async fn handle_wan_video_byok(request: &OmniRequest, app: &AppHandle) -> Response<TauriGenerateVideoResponse, TauriGenerateVideoErrorType, ()> {
  let credential_cache = app.state::<ProviderCredentialLoadingCache>();
  let task_database = app.state::<TaskDatabase>();

  match handle_wan_video(request, &credential_cache).await {
    Ok(success) => {
      if let Err(err) = success.insert_into_task_database_with_frontend_payload(&task_database, request.frontend_caller, request.frontend_subscriber_id.as_deref(), request.frontend_subscriber_payload.as_deref()).await {
        error!("Could not persist accepted Wan generation: {:?}", err);
      }

      let event = GenerationEnqueueSuccessEvent { action: success.to_frontend_event_action(), service: success.to_frontend_event_service(), model: success.model };
      if let Err(err) = event.send(app) {
        error!("Failed to emit enqueue event: {:?}", err);
      }
      CreditsBalanceChangedEvent {}.send_infallible(app);

      Ok(TauriGenerateVideoResponse {}.into())
    },
    Err(err) => {
      error!("Wan BYOK generation failed: {:?}", err);
      notify_frontend_of_errors(app, &err).await;

      let (status, error_type, message) = match &err {
        GenerateError::BadInput(BadInputReason::NoModelSpecified) => (CommandErrorStatus::BadRequest, TauriGenerateVideoErrorType::ModelNotSpecified, "No model specified for Wan video generation".to_string()),
        GenerateError::MissingCredentials(_) => (CommandErrorStatus::Unauthorized, TauriGenerateVideoErrorType::ServerError, "Wan base URL or API key is not configured".to_string()),
        _ => (CommandErrorStatus::ServerError, TauriGenerateVideoErrorType::ServerError, format!("Wan generation failed: {:?}", err)),
      };

      Err(CommandErrorResponseWrapper { status, error_message: Some(message), error_type: Some(error_type), error_details: None })
    },
  }
}

// ── Helpers ──

fn load_credential_string(credential_cache: &ProviderCredentialLoadingCache, key: ProviderCredentialKey) -> Option<String> {
  match credential_cache.get_credentials(key) {
    Ok(Some(ProviderCredentialPayload::ApiKey(data))) => Some(data.as_str().to_string()),
    _ => None,
  }
}

/// Map omni request fields to the gateway `parameters` object. Best-effort:
/// keys are read from both their omni and gateway spellings, and the ratio and
/// resolution are translated because the BYOK path bypasses `artcraft_router`.
/// A value with no upstream equivalent is omitted so the gateway applies its
/// own default rather than rejecting the request.
fn build_parameters(fields: &serde_json::Map<String, Value>) -> VideoParameters {
  VideoParameters { resolution: fields.get("resolution").and_then(Value::as_str).and_then(wire_resolution).map(str::to_owned), ratio: fields.get("aspect_ratio").or_else(|| fields.get("ratio")).and_then(Value::as_str).and_then(wire_ratio).map(str::to_owned), duration: fields.get("duration_seconds").or_else(|| fields.get("duration")).and_then(Value::as_i64).map(|v| v as i32), audio: fields.get("generate_audio").or_else(|| fields.get("audio")).and_then(Value::as_bool), seed: fields.get("seed").and_then(Value::as_i64) }
}
