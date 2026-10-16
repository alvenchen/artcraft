use log::{error, info, warn};
use seedance_client::api::requests::video_generation::{query_video_task, SeedanceTaskStatus};
use seedance_client::creds::seedance_api_key::SeedanceApiKey;
use seedance_client::creds::seedance_base_url::SeedanceBaseUrl;
use sqlite_tasks::queries::task::Task;
use tauri::AppHandle;

use crate::core::providers::credentials::payload::provider_credential_payload::ProviderCredentialPayload;
use crate::core::providers::credentials::provider_credential_key::ProviderCredentialKey;
use crate::core::providers::credentials::provider_credential_loading_cache::ProviderCredentialLoadingCache;
use crate::core::providers::selected_upstream_providers::SEEDANCE_PROVIDER;
use crate::core::state::app_env_configs::app_env_configs::AppEnvConfigs;
use crate::core::state::data_dir::app_data_root::AppDataRoot;
use crate::core::state::task_database::TaskDatabase;
use crate::core::threads::third_party_task_polling_thread::handlers::seedance::handle_seedance_complete::handle_seedance_complete;
use crate::core::threads::third_party_task_polling_thread::handlers::seedance::handle_seedance_failure::handle_seedance_failure;
use crate::services::storyteller::state::storyteller_credential_manager::StorytellerCredentialManager;

pub async fn poll_seedance_tasks(app_handle: &AppHandle, app_env_configs: &AppEnvConfigs, app_data_root: &AppDataRoot, task_database: &TaskDatabase, storyteller_creds_manager: &StorytellerCredentialManager, credential_cache: &ProviderCredentialLoadingCache, seedance_tasks: &[&Task]) {
  let (base_url, api_key) = match load_seedance_credentials(credential_cache) {
    Some(creds) => creds,
    None => {
      warn!("[SeedancePolling] Seedance credentials not configured, skipping poll");
      return;
    },
  };

  for task in seedance_tasks {
    let result = poll_single_seedance_task(app_handle, app_env_configs, app_data_root, task_database, storyteller_creds_manager, &base_url, &api_key, task).await;

    if let Err(err) = result {
      error!("[SeedancePolling] Error processing task {}: {:?}", task.id.as_str(), err);
    }
  }
}

async fn poll_single_seedance_task(app_handle: &AppHandle, app_env_configs: &AppEnvConfigs, app_data_root: &AppDataRoot, task_database: &TaskDatabase, storyteller_creds_manager: &StorytellerCredentialManager, base_url: &SeedanceBaseUrl, api_key: &SeedanceApiKey, task: &Task) -> Result<(), Box<dyn std::error::Error>> {
  let task_id = match &task.provider_job_id {
    Some(id) => id.as_str(),
    None => {
      warn!("[SeedancePolling] Task {} has no provider job id, marking as failed", task.id.as_str());
      handle_seedance_failure(app_handle, task_database, task, "No Seedance task id to check").await;
      return Ok(());
    },
  };

  let status = query_video_task(base_url, api_key, task_id).await?;

  match status {
    SeedanceTaskStatus::Pending | SeedanceTaskStatus::Running => {
      info!("[SeedancePolling] Task {} still in progress", task.id.as_str());
    },
    SeedanceTaskStatus::Unknown => {
      warn!("[SeedancePolling] Task {} has unknown status, skipping", task.id.as_str());
    },
    SeedanceTaskStatus::Succeeded { video_url } => {
      info!("[SeedancePolling] Task {} succeeded", task.id.as_str());
      handle_seedance_complete(app_handle, app_env_configs, app_data_root, task_database, storyteller_creds_manager, task, &video_url).await;
    },
    SeedanceTaskStatus::Failed { message } => {
      let reason = message.unwrap_or_else(|| "Seedance generation failed".to_string());
      handle_seedance_failure(app_handle, task_database, task, &reason).await;
    },
    SeedanceTaskStatus::Expired => {
      handle_seedance_failure(app_handle, task_database, task, "Seedance task expired").await;
    },
  }

  Ok(())
}

fn load_seedance_credentials(credential_cache: &ProviderCredentialLoadingCache) -> Option<(SeedanceBaseUrl, SeedanceApiKey)> {
  let base_url = load_credential_string(credential_cache, ProviderCredentialKey::SeedanceBaseUrl)?;
  let api_key = load_credential_string(credential_cache, ProviderCredentialKey::SeedanceApiKey)?;
  Some((SeedanceBaseUrl::new(base_url, SEEDANCE_PROVIDER), SeedanceApiKey::new(api_key)))
}

fn load_credential_string(credential_cache: &ProviderCredentialLoadingCache, key: ProviderCredentialKey) -> Option<String> {
  match credential_cache.get_credentials(key) {
    Ok(Some(ProviderCredentialPayload::ApiKey(data))) => Some(data.as_str().to_string()),
    _ => None,
  }
}
