use artcraft_api_defs::utils::media_links_to_thumbnail_template::media_links_to_thumbnail_template;
use artcraft_client::endpoints::media_files::get_media_file::get_media_file;
use artcraft_client::endpoints::media_files::upload_video_media_file_from_file::{upload_video_media_file_from_file, UploadVideoFromFileArgs};
use enums::common::generation_provider::GenerationProvider;
use enums::tauri::tasks::task_media_file_class::TaskMediaFileClass;
use log::{error, info};
use sqlite_tasks::queries::task::Task;
use sqlite_tasks::queries::update_successful_task_status_with_metadata::{update_successful_task_status_with_metadata, UpdateSuccessfulTaskArgs};
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use tauri::AppHandle;
use tokens::tokens::media_files::MediaFileToken;
use tokens::tokens::prompts::PromptToken;
use uuid_utils::uuid::generate_random_uuid;

use crate::core::state::app_env_configs::app_env_configs::AppEnvConfigs;
use crate::core::state::data_dir::app_data_root::AppDataRoot;
use crate::core::state::data_dir::trait_data_subdir::DataSubdir;
use crate::core::state::task_database::TaskDatabase;
use crate::core::threads::third_party_task_polling_thread::events::notify_frontend_of_completion::{notify_frontend_of_completion, CompletionData};
use crate::core::threads::third_party_task_polling_thread::handlers::seedance::handle_seedance_failure::handle_seedance_failure;
use crate::core::utils::auto_download::{auto_download_task_urls, clear_auto_download_checkpoint};
use crate::services::storyteller::state::storyteller_credential_manager::StorytellerCredentialManager;

/// Finalize a succeeded Seedance task: download the ARK `video_url`, upload it
/// to the Artcraft backend (so the result appears in the gallery/generation
/// list), and notify the frontend. Mirrors the Wan completion flow.
pub async fn handle_seedance_complete(app_handle: &AppHandle, app_env_configs: &AppEnvConfigs, app_data_root: &AppDataRoot, task_database: &TaskDatabase, storyteller_creds_manager: &StorytellerCredentialManager, task: &Task, video_url: &str) {
  let result = handle_seedance_complete_inner(app_handle, app_env_configs, app_data_root, task_database, storyteller_creds_manager, task, video_url).await;

  if let Err(err) = result {
    error!("[SeedanceComplete] Failed to handle task {}: {:?}", task.id.as_str(), err);
  }
}

async fn handle_seedance_complete_inner(app_handle: &AppHandle, app_env_configs: &AppEnvConfigs, app_data_root: &AppDataRoot, task_database: &TaskDatabase, storyteller_creds_manager: &StorytellerCredentialManager, task: &Task, video_url: &str) -> Result<(), Box<dyn std::error::Error>> {
  if video_url.trim().is_empty() {
    handle_seedance_failure(app_handle, task_database, task, "Seedance task succeeded with no video URL").await;
    return Ok(());
  }

  let url = reqwest::Url::parse(video_url)?;

  // Save a local copy before uploading or announcing completion.
  auto_download_task_urls(app_handle, task, std::slice::from_ref(&url)).await?;

  let creds = storyteller_creds_manager.get_credentials()?.ok_or("No Storyteller credentials available to save the Seedance result")?;

  let maybe_prompt_token = task.prompt_token.as_ref().map(|s| PromptToken::new_from_str(s));

  let download_path = download_file(video_url, app_data_root).await?;

  info!("[SeedanceComplete] Uploading result for task {} to backend...", task.id.as_str());

  let media_token = upload_video_media_file_from_file(UploadVideoFromFileArgs { api_host: &app_env_configs.storyteller_host, maybe_creds: Some(&creds), path: &download_path, maybe_prompt_token: maybe_prompt_token.as_ref(), maybe_generation_provider: Some(GenerationProvider::Seedance) }).await?.media_file_token;

  info!("[SeedanceComplete] Uploaded as media file: {:?}", media_token);

  let (maybe_cdn_url, maybe_thumbnail_url_template) = look_up_media(app_env_configs, &media_token).await;

  let updated = update_successful_task_status_with_metadata(UpdateSuccessfulTaskArgs { db: task_database.get_connection(), task_id: &task.id, maybe_batch_token: None, maybe_primary_media_file_token: Some(&media_token), maybe_primary_media_file_class: Some(TaskMediaFileClass::Video), maybe_primary_media_file_cdn_url: maybe_cdn_url.as_ref().map(|u| u.as_str()), maybe_primary_media_file_thumbnail_url_template: maybe_thumbnail_url_template.as_deref() }).await?;

  if updated {
    clear_auto_download_checkpoint(app_handle, task);

    let completion = CompletionData { primary_media_file_token: media_token, maybe_cdn_url, maybe_thumbnail_url_template, maybe_batch_token: None, media_class: TaskMediaFileClass::Video };

    notify_frontend_of_completion(app_handle, &app_env_configs.storyteller_host, Some(&creds), task, &completion).await;
  }

  info!("[SeedanceComplete] Task {} fully handled", task.id.as_str());

  Ok(())
}

// ── Helpers ──

async fn download_file(video_url: &str, app_data_root: &AppDataRoot) -> Result<PathBuf, Box<dyn std::error::Error>> {
  let response = reqwest::get(video_url).await?;
  let bytes = response.bytes().await?;

  let extension = url_utils::download_extension::extract_download_extension_from_url::extract_download_extension_from_url_str(video_url).map(|ext| ext.as_extension_without_period()).unwrap_or("mp4");

  let tempdir = app_data_root.temp_dir().path();
  let filename = format!("seedance_{}.{}", generate_random_uuid(), extension);
  let download_path = tempdir.join(filename);

  let mut file = File::create(&download_path)?;
  file.write_all(&bytes)?;

  Ok(download_path)
}

async fn look_up_media(app_env_configs: &AppEnvConfigs, media_token: &MediaFileToken) -> (Option<reqwest::Url>, Option<String>) {
  match get_media_file(&app_env_configs.storyteller_host, media_token).await {
    Ok(response) => {
      let cdn_url = response.media_file.media_links.cdn_url.clone();
      let thumbnail = media_links_to_thumbnail_template(&response.media_file.media_links).map(|s| s.to_string());
      (Some(cdn_url), thumbnail)
    },
    Err(err) => {
      error!("[SeedanceComplete] Failed to look up media file after upload (failing open): {:?}", err);
      (None, None)
    },
  }
}
