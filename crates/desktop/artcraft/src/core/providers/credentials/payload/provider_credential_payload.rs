use crate::core::providers::credentials::payload::api_key::ApiKeyData;

#[derive(Clone)]
pub enum ProviderCredentialPayload {
  ApiKey(ApiKeyData),
}
