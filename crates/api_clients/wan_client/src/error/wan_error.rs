use std::error::Error;

use crate::error::wan_api_error::WanApiError;
use crate::error::wan_client_error::WanClientError;

/// Top-level error for the Wan-AI client. Sum of every failure mode any
/// endpoint can produce.
#[derive(Debug)]
pub enum WanError {
  Client(WanClientError),
  Api(WanApiError),
}

impl Error for WanError {}

impl std::fmt::Display for WanError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Client(e) => write!(f, "WanClientError: {:?}", e),
      Self::Api(e) => write!(f, "WanApiError: {:?}", e),
    }
  }
}

impl From<WanClientError> for WanError {
  fn from(error: WanClientError) -> Self {
    Self::Client(error)
  }
}

impl From<WanApiError> for WanError {
  fn from(error: WanApiError) -> Self {
    Self::Api(error)
  }
}
