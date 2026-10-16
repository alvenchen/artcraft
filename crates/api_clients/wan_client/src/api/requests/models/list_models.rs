use serde_derive::{Deserialize, Serialize};

use crate::creds::wan_api_key::WanApiKey;
use crate::creds::wan_base_url::WanBaseUrl;
use crate::error::wan_api_error::WanApiError;
use crate::error::wan_client_error::WanClientError;
use crate::error::wan_error::WanError;

/// Only model ids containing this substring (case-insensitive) are kept, so a
/// gateway that also proxies other vendors' models doesn't pollute the picker.
const WAN_MODEL_ID_FILTER: &str = "wan";

// ── Public args ──

/// Top-level argument to [`list_models`]. Borrows the base URL and API key so
/// callers can log without leaking the credential.
#[derive(Clone, Debug)]
pub struct ListModelsArgs<'a> {
  pub base_url: &'a WanBaseUrl,
  pub api_key: &'a WanApiKey,
}

// ── Response ──

/// The material model info we surface to the app.
#[derive(Clone, Debug, Serialize)]
pub struct WanModelSummary {
  pub id: String,
}

/// OpenAI-style `GET /models` response body.
#[derive(Deserialize, Debug)]
struct ListModelsResponseBody {
  data: Vec<ListModelsModelEntry>,
}

#[derive(Deserialize, Debug)]
struct ListModelsModelEntry {
  id: String,
}

/// List the Wan models exposed by the user's gateway (`GET {base}/models`),
/// filtered to ids containing "wan".
pub async fn list_models(args: ListModelsArgs<'_>) -> Result<Vec<WanModelSummary>, WanError> {
  let url = args.base_url.models_url();

  let client = reqwest::Client::builder().build().map_err(WanClientError::ReqwestClientError)?;

  let response = client.get(&url).bearer_auth(args.api_key.as_str()).send().await.map_err(WanApiError::ReqwestError)?;

  let status = response.status();
  let body = response.text().await.map_err(WanApiError::ReqwestError)?;

  if !status.is_success() {
    return Err(WanApiError::from_status_and_body(status, body).into());
  }

  let parsed: ListModelsResponseBody = serde_json::from_str(&body).map_err(|err| WanApiError::SerdeResponseParseErrorWithBody(err, body.clone()))?;

  let models = parsed.data.into_iter().filter(|entry| entry.id.to_lowercase().contains(WAN_MODEL_ID_FILTER)).map(|entry| WanModelSummary { id: entry.id }).collect();

  Ok(models)
}
