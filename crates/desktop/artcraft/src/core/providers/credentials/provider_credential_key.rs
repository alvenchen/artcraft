use serde_derive::{Deserialize, Serialize};
use crate::core::providers::credentials::provider_credential_type::ProviderCredentialType;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderCredentialKey {

  FalApiKey,
  ReplicateApiKey,
  GmiCloudApiKey,
  GrokApiKey,
}


impl ProviderCredentialKey {
  pub fn get_type(&self) -> ProviderCredentialType {
    match self {
      Self::FalApiKey => ProviderCredentialType::ApiKey,
      Self::ReplicateApiKey => ProviderCredentialType::ApiKey,
      Self::GmiCloudApiKey => ProviderCredentialType::ApiKey,
      Self::GrokApiKey => ProviderCredentialType::ApiKey,
    }
  }

  pub fn get_filename(&self) -> &'static str {
    match self {
      ProviderCredentialKey::FalApiKey => "fal.api_key.txt",
      ProviderCredentialKey::ReplicateApiKey => "replicate.api_key.txt",
      ProviderCredentialKey::GmiCloudApiKey => "gmicloud.api_key.txt",
      ProviderCredentialKey::GrokApiKey => "grok.api_key.txt",
    }
  }
}
