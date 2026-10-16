/// Holds the API key for authenticating with the user's Wan-AI gateway.
///
/// Sent on every request as `Authorization: Bearer <api_key>`.
#[derive(Clone)]
pub struct WanApiKey {
  api_key: String,
}

impl WanApiKey {
  pub fn new(api_key: impl Into<String>) -> Self {
    Self { api_key: api_key.into().trim().to_string() }
  }

  pub fn as_str(&self) -> &str {
    &self.api_key
  }
}

// Debug is redacted so the key can't end up in a log line via accidental
// `{:?}` on an enclosing args struct.
impl std::fmt::Debug for WanApiKey {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "WanApiKey(<redacted>)")
  }
}
