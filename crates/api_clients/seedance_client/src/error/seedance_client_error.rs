use std::error::Error;
use std::fmt::{Display, Formatter};

/// Errors that happen entirely on the client side — before the HTTP request
/// reaches the ARK API.
#[derive(Debug)]
pub enum SeedanceClientError {
  /// An error was encountered building the reqwest client.
  ReqwestClientError(reqwest::Error),

  /// The request body could not be serialized to JSON.
  RequestSerializationError(serde_json::Error),
}

impl Error for SeedanceClientError {}

impl Display for SeedanceClientError {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::ReqwestClientError(err) => write!(f, "Reqwest client error (during client creation): {}", err),
      Self::RequestSerializationError(err) => write!(f, "Failed to serialize request body to JSON: {}", err),
    }
  }
}
