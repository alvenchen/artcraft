// Provider credential keys. Defined in Rust (`ProviderCredentialKey`) as the
// source of truth and duplicated here. Add new providers here as they gain
// API-key support.

export enum ProviderCredentialKey {
  FalApiKey = "fal_api_key",
  ReplicateApiKey = "replicate_api_key",
  GmiCloudApiKey = "gmicloud_api_key",
  GrokApiKey = "grok_api_key",
}

export enum ProviderCredentialType {
  ApiKey = "api_key",
}
