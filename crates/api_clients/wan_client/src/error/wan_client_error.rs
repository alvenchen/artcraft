use std::error::Error;
use std::fmt::{Display, Formatter};

/// Errors that happen entirely on the client side — before the HTTP request
/// reaches the gateway.
#[derive(Debug)]
pub enum WanClientError {
  /// No base URL is configured.
  NoBaseUrlPresent,

  /// No API key is present.
  NoApiKeyPresent,

  /// An error was encountered building the reqwest client.
  ReqwestClientError(reqwest::Error),

  /// The request body could not be serialized to JSON.
  RequestSerializationError(serde_json::Error),

  /// The request failed our own validation before being sent — e.g. a required
  /// input missing or an oversize payload. No HTTP call was made.
  InvalidRequest(String),
}

impl Error for WanClientError {}

impl Display for WanClientError {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::NoBaseUrlPresent => write!(f, "No Wan base URL configured."),
      Self::NoApiKeyPresent => write!(f, "No Wan API key present."),
      Self::ReqwestClientError(err) => write!(f, "Reqwest client error (during client creation): {}", err),
      Self::RequestSerializationError(err) => write!(f, "Failed to serialize request body to JSON: {}", err),
      Self::InvalidRequest(msg) => write!(f, "Invalid request (rejected client-side before sending): {}", msg),
    }
  }
}
