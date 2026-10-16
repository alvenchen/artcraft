use serde_derive::{Deserialize, Serialize};
use crate::core::providers::credentials::provider_credential_type::ProviderCredentialType;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderCredentialKey {
  FalApiKey,
  ReplicateApiKey,
  GmiCloudApiKey,
  GrokApiKey,

  // Wan-AI is user-hosted: the base URL is configuration, the API key is a secret.
  WanBaseUrl,
  WanApiKey,

  // ByteDance Seedance via Volcano Engine ARK (or BytePlus). The base URL is
  // user-configurable because the two deployments use different hosts.
  SeedanceBaseUrl,
  SeedanceApiKey,
}

impl ProviderCredentialKey {
  pub fn get_type(&self) -> ProviderCredentialType {
    match self {
      Self::FalApiKey => ProviderCredentialType::ApiKey,
      Self::ReplicateApiKey => ProviderCredentialType::ApiKey,
      Self::GmiCloudApiKey => ProviderCredentialType::ApiKey,
      Self::GrokApiKey => ProviderCredentialType::ApiKey,
      Self::WanBaseUrl => ProviderCredentialType::ApiKey,
      Self::WanApiKey => ProviderCredentialType::ApiKey,
      Self::SeedanceBaseUrl => ProviderCredentialType::ApiKey,
      Self::SeedanceApiKey => ProviderCredentialType::ApiKey,
    }
  }

  /// Whether the stored value is a secret that should be redacted in listings.
  /// Base URLs are configuration, not secrets.
  pub fn is_secret(&self) -> bool {
    !matches!(self, Self::WanBaseUrl | Self::SeedanceBaseUrl)
  }

  pub fn get_filename(&self) -> &'static str {
    match self {
      ProviderCredentialKey::FalApiKey => "fal.api_key.txt",
      ProviderCredentialKey::ReplicateApiKey => "replicate.api_key.txt",
      ProviderCredentialKey::GmiCloudApiKey => "gmicloud.api_key.txt",
      ProviderCredentialKey::GrokApiKey => "grok.api_key.txt",
      ProviderCredentialKey::WanBaseUrl => "wan.base_url.txt",
      ProviderCredentialKey::WanApiKey => "wan.api_key.txt",
      ProviderCredentialKey::SeedanceBaseUrl => "seedance.base_url.txt",
      ProviderCredentialKey::SeedanceApiKey => "seedance.api_key.txt",
    }
  }
}
