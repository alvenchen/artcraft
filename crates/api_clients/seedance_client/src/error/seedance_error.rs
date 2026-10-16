use std::error::Error;

use crate::error::seedance_api_error::SeedanceApiError;
use crate::error::seedance_client_error::SeedanceClientError;

/// Top-level error for the Seedance client. Sum of every failure mode any
/// endpoint can produce.
#[derive(Debug)]
pub enum SeedanceError {
  Client(SeedanceClientError),
  Api(SeedanceApiError),
}

impl Error for SeedanceError {}

impl std::fmt::Display for SeedanceError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Client(e) => write!(f, "SeedanceClientError: {:?}", e),
      Self::Api(e) => write!(f, "SeedanceApiError: {:?}", e),
    }
  }
}

impl From<SeedanceClientError> for SeedanceError {
  fn from(error: SeedanceClientError) -> Self {
    Self::Client(error)
  }
}

impl From<SeedanceApiError> for SeedanceError {
  fn from(error: SeedanceApiError) -> Self {
    Self::Api(error)
  }
}
