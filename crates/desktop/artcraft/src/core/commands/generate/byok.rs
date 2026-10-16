use log::info;

use enums::common::generation_provider::GenerationProvider;
use seedance_client::api::requests::seedance_model_id::seedance_model_id;

use crate::core::providers::credentials::provider_credential_key::ProviderCredentialKey;
use crate::core::providers::credentials::provider_credential_loading_cache::ProviderCredentialLoadingCache;
use crate::core::state::app_preferences::app_preferences_manager::AppPreferencesManager;

/// A model that can be generated directly against the user's own provider
/// credentials (Settings → API Keys) when BYOK mode is on and the keys exist.
pub struct ByokRoute {
  pub provider: GenerationProvider,
  pub required_keys: &'static [ProviderCredentialKey],
}

/// Map a model id to the provider that owns it for BYOK routing.
///
/// Only providers with a direct generation handler are listed. Everything else
/// returns `None` and keeps going through the Artcraft backend.
pub fn resolve_byok_route(model: &str) -> Option<ByokRoute> {
  if model.starts_with("wan_") {
    return Some(ByokRoute { provider: GenerationProvider::Wan, required_keys: &[ProviderCredentialKey::WanBaseUrl, ProviderCredentialKey::WanApiKey] });
  }

  // Seedance is routed only when the slug maps to a known Volcano Engine model.
  // BytePlus (`*_bp*` / `*_bpu*`) and unmapped slugs return `None` and keep
  // going through the Artcraft backend.
  if model.starts_with("seedance_") && seedance_model_id(model).is_some() {
    return Some(ByokRoute { provider: GenerationProvider::Seedance, required_keys: &[ProviderCredentialKey::SeedanceBaseUrl, ProviderCredentialKey::SeedanceApiKey] });
  }

  None
}

/// Resolve a BYOK route only when the global toggle is on and every required
/// key is configured. Returns `None` to fall back to the Artcraft backend.
pub fn resolve_if_configured(model: Option<&str>, prefs: &AppPreferencesManager, credential_cache: &ProviderCredentialLoadingCache) -> Option<ByokRoute> {
  let prefs = prefs.get_clone().ok()?;
  info!("resolve_if_configured: use_own_api_keys={}", prefs.use_own_api_keys);

  if !prefs.use_own_api_keys {
    return None;
  }

  let model = model?;
  info!("resolve_if_configured: model={:?}", model);

  let route = resolve_byok_route(model)?;
  for &key in route.required_keys {
    let maybe_payload = credential_cache.get_credentials(key);
    info!("resolve_if_configured: key={:?}, filename={:?}, present={}, error={:?}", key, key.get_filename(), maybe_payload.as_ref().ok().and_then(Option::as_ref).is_some(), maybe_payload.as_ref().err(),);

    if maybe_payload.ok().flatten().is_none() {
      info!("resolve_if_configured: falling back, missing {:?}", key);
      return None;
    }
  }

  Some(route)
}
