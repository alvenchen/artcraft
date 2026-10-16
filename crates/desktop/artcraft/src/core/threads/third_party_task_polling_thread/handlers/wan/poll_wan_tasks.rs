use log::{error, info, warn};
use sqlite_tasks::queries::task::Task;
use tauri::AppHandle;
use wan_client::api::requests::video_generation::{query_video_task, WanTaskStatus};
use wan_client::creds::wan_api_key::WanApiKey;
use wan_client::creds::wan_base_url::WanBaseUrl;

use crate::core::providers::credentials::payload::provider_credential_payload::ProviderCredentialPayload;
use crate::core::providers::credentials::provider_credential_key::ProviderCredentialKey;
use crate::core::providers::credentials::provider_credential_loading_cache::ProviderCredentialLoadingCache;
use crate::core::providers::selected_upstream_providers::WAN_PROVIDER;
use crate::core::state::app_env_configs::app_env_configs::AppEnvConfigs;
use crate::core::state::data_dir::app_data_root::AppDataRoot;
use crate::core::state::task_database::TaskDatabase;
use crate::core::threads::third_party_task_polling_thread::handlers::wan::handle_wan_complete::handle_wan_complete;
use crate::core::threads::third_party_task_polling_thread::handlers::wan::handle_wan_failure::handle_wan_failure;
use crate::services::storyteller::state::storyteller_credential_manager::StorytellerCredentialManager;

pub async fn poll_wan_tasks(app_handle: &AppHandle, app_env_configs: &AppEnvConfigs, app_data_root: &AppDataRoot, task_database: &TaskDatabase, storyteller_creds_manager: &StorytellerCredentialManager, credential_cache: &ProviderCredentialLoadingCache, wan_tasks: &[&Task]) {
  let (base_url, api_key) = match load_wan_credentials(credential_cache) {
    Some(creds) => creds,
    None => {
      warn!("[WanPolling] Wan credentials not configured, skipping poll");
      return;
    },
  };

  for task in wan_tasks {
    let result = poll_single_wan_task(app_handle, app_env_configs, app_data_root, task_database, storyteller_creds_manager, &base_url, &api_key, task).await;

    if let Err(err) = result {
      error!("[WanPolling] Error processing task {}: {:?}", task.id.as_str(), err);
    }
  }
}

async fn poll_single_wan_task(app_handle: &AppHandle, app_env_configs: &AppEnvConfigs, app_data_root: &AppDataRoot, task_database: &TaskDatabase, storyteller_creds_manager: &StorytellerCredentialManager, base_url: &WanBaseUrl, api_key: &WanApiKey, task: &Task) -> Result<(), Box<dyn std::error::Error>> {
  let task_id = match &task.provider_job_id {
    Some(id) => id.as_str(),
    None => {
      warn!("[WanPolling] Task {} has no provider job id, marking as failed", task.id.as_str());
      handle_wan_failure(app_handle, task_database, task, "No Wan task id to check").await;
      return Ok(());
    },
  };

  let status = query_video_task(base_url, api_key, task_id).await?;

  match status {
    WanTaskStatus::Pending | WanTaskStatus::Running => {
      info!("[WanPolling] Task {} still in progress", task.id.as_str());
    },
    WanTaskStatus::Unknown => {
      warn!("[WanPolling] Task {} has unknown status, skipping", task.id.as_str());
    },
    WanTaskStatus::Succeeded { video_url } => {
      info!("[WanPolling] Task {} succeeded", task.id.as_str());
      handle_wan_complete(app_handle, app_env_configs, app_data_root, task_database, storyteller_creds_manager, task, &video_url).await;
    },
    WanTaskStatus::Failed { code, message } => {
      let reason = message.or(code).unwrap_or_else(|| "Wan generation failed".to_string());
      handle_wan_failure(app_handle, task_database, task, &reason).await;
    },
    WanTaskStatus::Canceled => {
      handle_wan_failure(app_handle, task_database, task, "Wan task was canceled").await;
    },
  }

  Ok(())
}

fn load_wan_credentials(credential_cache: &ProviderCredentialLoadingCache) -> Option<(WanBaseUrl, WanApiKey)> {
  let base_url = load_credential_string(credential_cache, ProviderCredentialKey::WanBaseUrl)?;
  let api_key = load_credential_string(credential_cache, ProviderCredentialKey::WanApiKey)?;
  Some((WanBaseUrl::new(base_url, WAN_PROVIDER), WanApiKey::new(api_key)))
}

fn load_credential_string(credential_cache: &ProviderCredentialLoadingCache, key: ProviderCredentialKey) -> Option<String> {
  match credential_cache.get_credentials(key) {
    Ok(Some(ProviderCredentialPayload::ApiKey(data))) => Some(data.as_str().to_string()),
    _ => None,
  }
}
