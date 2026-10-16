use enums::common::generation_provider::GenerationProvider;
use enums::tauri::tasks::task_type::TaskType;
use log::{error, info};
use seedance_client::api::requests::seedance_model_id::seedance_model_id;
use seedance_client::api::requests::video_generation::{submit_video_generation, VideoGenerationArgs, VideoParameters};
use seedance_client::creds::seedance_api_key::SeedanceApiKey;
use seedance_client::creds::seedance_base_url::SeedanceBaseUrl;
use serde_json::Value;
use tauri::{AppHandle, Manager};
use tokens::tokens::media_files::MediaFileToken;

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
use crate::core::providers::selected_upstream_providers::SEEDANCE_PROVIDER;
use crate::core::state::app_env_configs::app_env_configs::AppEnvConfigs;
use crate::core::state::task_database::TaskDatabase;
use crate::core::utils::map_media_file_tokens_to_cdn_urls::map_media_file_tokens_to_cdn_urls;

/// Submit a Seedance video generation task against the user's own credentials
/// and return the enqueued task. The provider's task id becomes the provider
/// job id, polled by the third-party polling thread.
///
/// Keyframe images are referenced by CDN URL: the API fetches them itself, so
/// the tokens are resolved to links rather than uploaded.
pub async fn handle_seedance_video(request: &OmniRequest, credential_cache: &ProviderCredentialLoadingCache, app_env_configs: &AppEnvConfigs) -> Result<TaskEnqueueSuccess, GenerateError> {
  let model = request.model().ok_or_else(GenerateError::no_model_specified)?;
  let doubao_model = seedance_model_id(model).ok_or_else(|| GenerateError::NotYetImplemented(format!("Seedance model {:?} is not supported", model)))?;

  let base_url = load_credential_string(credential_cache, ProviderCredentialKey::SeedanceBaseUrl).ok_or_else(GenerateError::needs_seedance_api_key)?;
  let api_key = load_credential_string(credential_cache, ProviderCredentialKey::SeedanceApiKey).ok_or_else(GenerateError::needs_seedance_api_key)?;

  let prompt = request.fields.get("prompt").and_then(Value::as_str).unwrap_or_default();

  let (maybe_start_frame_url, maybe_end_frame_url) = resolve_keyframe_urls(request, app_env_configs).await?;

  let base_url = SeedanceBaseUrl::new(base_url, SEEDANCE_PROVIDER);
  let api_key = SeedanceApiKey::new(api_key);

  info!("Submitting Seedance video generation via ARK: model={:?} baseURL={:?}", doubao_model, base_url.as_str());

  let submitted = submit_video_generation(VideoGenerationArgs { base_url: &base_url, api_key: &api_key, model: doubao_model, prompt, maybe_start_frame_url: maybe_start_frame_url.as_deref(), maybe_end_frame_url: maybe_end_frame_url.as_deref(), parameters: build_parameters(&request.fields) }).await.map_err(|err| {
    error!("Seedance video submission failed: {:?}", err);
    GenerateError::from(err)
  })?;

  Ok(TaskEnqueueSuccess { task_type: TaskType::VideoGeneration, model: Some(GenerationModel::Unknown(model.to_string())), provider: GenerationProvider::Seedance, provider_job_id: Some(submitted.task_id), maybe_queue_status_url: None, maybe_queue_response_url: None, maybe_prompt_token: None })
}

/// Command-level entry point for the BYOK path: submit, persist to the task
/// database, and emit the usual enqueue/credits events. Returns the same
/// response shape as the legacy native video path so callers can reuse
/// `adapt_legacy_response`.
pub async fn handle_seedance_video_byok(request: &OmniRequest, app: &AppHandle) -> Response<TauriGenerateVideoResponse, TauriGenerateVideoErrorType, ()> {
  let credential_cache = app.state::<ProviderCredentialLoadingCache>();
  let task_database = app.state::<TaskDatabase>();
  let app_env_configs = app.state::<AppEnvConfigs>();

  match handle_seedance_video(request, &credential_cache, &app_env_configs).await {
    Ok(success) => {
      if let Err(err) = success.insert_into_task_database_with_frontend_payload(&task_database, request.frontend_caller, request.frontend_subscriber_id.as_deref(), request.frontend_subscriber_payload.as_deref()).await {
        error!("Could not persist accepted Seedance generation: {:?}", err);
      }

      let event = GenerationEnqueueSuccessEvent { action: success.to_frontend_event_action(), service: success.to_frontend_event_service(), model: success.model };
      if let Err(err) = event.send(app) {
        error!("Failed to emit enqueue event: {:?}", err);
      }
      CreditsBalanceChangedEvent {}.send_infallible(app);

      Ok(TauriGenerateVideoResponse {}.into())
    },
    Err(err) => {
      error!("Seedance BYOK generation failed: {:?}", err);
      notify_frontend_of_errors(app, &err).await;

      let (status, error_type, message) = match &err {
        GenerateError::BadInput(BadInputReason::NoModelSpecified) => (CommandErrorStatus::BadRequest, TauriGenerateVideoErrorType::ModelNotSpecified, "No model specified for Seedance video generation".to_string()),
        GenerateError::MissingCredentials(_) => (CommandErrorStatus::Unauthorized, TauriGenerateVideoErrorType::ServerError, "Seedance base URL or API key is not configured".to_string()),
        _ => (CommandErrorStatus::ServerError, TauriGenerateVideoErrorType::ServerError, format!("Seedance generation failed: {:?}", err)),
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

/// Resolve the keyframe media tokens from the omni request into the CDN URLs
/// the API expects. The modern `start_frame_*` spellings are preferred; the
/// legacy `image_media_token` is read as a fallback for older callers.
async fn resolve_keyframe_urls(request: &OmniRequest, app_env_configs: &AppEnvConfigs) -> Result<(Option<String>, Option<String>), GenerateError> {
  let maybe_start_frame_token = media_token_field(&request.fields, "start_frame_image_media_token").or_else(|| media_token_field(&request.fields, "image_media_token"));
  let maybe_end_frame_token = media_token_field(&request.fields, "end_frame_image_media_token");

  Ok((resolve_media_token_url(maybe_start_frame_token, app_env_configs).await?, resolve_media_token_url(maybe_end_frame_token, app_env_configs).await?))
}

async fn resolve_media_token_url(maybe_token: Option<MediaFileToken>, app_env_configs: &AppEnvConfigs) -> Result<Option<String>, GenerateError> {
  let Some(token) = maybe_token else {
    return Ok(None);
  };

  let mut urls = map_media_file_tokens_to_cdn_urls(std::slice::from_ref(&token), &app_env_configs.storyteller_host).await?;
  Ok(urls.pop())
}

fn media_token_field(fields: &serde_json::Map<String, Value>, key: &str) -> Option<MediaFileToken> {
  fields.get(key).and_then(Value::as_str).filter(|value| !value.is_empty()).map(MediaFileToken::new_from_str)
}

fn build_parameters(fields: &serde_json::Map<String, Value>) -> VideoParameters {
  VideoParameters { resolution: fields.get("resolution").and_then(Value::as_str).and_then(wire_resolution).map(str::to_owned), ratio: fields.get("aspect_ratio").or_else(|| fields.get("ratio")).and_then(Value::as_str).and_then(wire_ratio).map(str::to_owned), duration: fields.get("duration_seconds").or_else(|| fields.get("duration")).and_then(Value::as_i64).map(|v| v as i32), seed: fields.get("seed").and_then(Value::as_i64), generate_audio: fields.get("generate_audio").or_else(|| fields.get("audio")).and_then(Value::as_bool), output_format: fields.get("output_format").and_then(Value::as_str).map(str::to_owned) }
}
