use log::{info, warn};
use serde_derive::Serialize;
use tauri::State;
use wan_client::api::requests::models::list_models::{list_models, ListModelsArgs};
use wan_client::creds::wan_api_key::WanApiKey;
use wan_client::creds::wan_base_url::WanBaseUrl;

use crate::core::commands::response::shorthand::ResponseOrErrorMessage;
use crate::core::commands::response::success_response_wrapper::SerializeMarker;
use crate::core::providers::credentials::payload::provider_credential_payload::ProviderCredentialPayload;
use crate::core::providers::credentials::provider_credential_key::ProviderCredentialKey;
use crate::core::providers::credentials::provider_credential_loading_cache::ProviderCredentialLoadingCache;
use crate::core::providers::selected_upstream_providers::WAN_PROVIDER;

#[derive(Serialize)]
pub struct WanListModelsResponse {
  pub models: Vec<WanModelEntry>,
}

impl SerializeMarker for WanListModelsResponse {}

#[derive(Serialize)]
pub struct WanModelEntry {
  pub id: String,
}

/// List the Wan models exposed by the user's configured gateway
/// (`GET {base}/models`, filtered to ids containing "wan").
#[tauri::command]
pub async fn wan_list_models_command(credential_cache: State<'_, ProviderCredentialLoadingCache>) -> ResponseOrErrorMessage<WanListModelsResponse> {
  info!("wan_list_models_command called");

  let base_url = match load_credential_string(&credential_cache, ProviderCredentialKey::WanBaseUrl) {
    Some(value) => value,
    None => return Err("Wan base URL is not configured".into()),
  };

  let api_key = match load_credential_string(&credential_cache, ProviderCredentialKey::WanApiKey) {
    Some(value) => value,
    None => return Err("Wan API key is not configured".into()),
  };

  let base_url = WanBaseUrl::new(base_url, WAN_PROVIDER);
  let api_key = WanApiKey::new(api_key);

  match list_models(ListModelsArgs { base_url: &base_url, api_key: &api_key }).await {
    Ok(models) => {
      let models = models.into_iter().map(|model| WanModelEntry { id: model.id }).collect();
      Ok(WanListModelsResponse { models }.into())
    },
    Err(err) => {
      warn!("wan_list_models_command failed: {:?}", err);
      Err(format!("Failed to list Wan models: {}", err).into())
    },
  }
}

fn load_credential_string(credential_cache: &ProviderCredentialLoadingCache, key: ProviderCredentialKey) -> Option<String> {
  match credential_cache.get_credentials(key) {
    Ok(Some(ProviderCredentialPayload::ApiKey(data))) => Some(data.as_str().to_string()),
    _ => None,
  }
}
