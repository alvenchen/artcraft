use std::error::Error;
use std::fmt::{Display, Formatter};

use reqwest::StatusCode;

/// Errors from the gateway's HTTP responses.
#[derive(Debug)]
pub enum WanApiError {
  /// 401 Unauthorized — API key is missing, invalid, or revoked.
  Unauthorized { raw_http_body: Option<String> },

  /// 403 Forbidden — the key is valid but lacks permission for this resource.
  Forbidden { raw_http_body: Option<String> },

  /// 404 Not Found — typically an unknown model or task id.
  NotFound { raw_http_body: Option<String> },

  /// 429 Too Many Requests — rate limit exceeded.
  RateLimited { raw_http_body: Option<String> },

  /// 400 Bad Request — the request body shape was invalid.
  BadRequest { reason: String, raw_http_body: Option<String> },

  /// Response body didn't deserialize against the expected schema. Includes
  /// the original body so we can diagnose drift.
  SerdeResponseParseErrorWithBody(serde_json::Error, String),

  /// An uncategorized bad HTTP response with status code and body.
  UncategorizedBadResponseWithStatusAndBody { status_code: StatusCode, body: String },

  /// An uncaught error from the HTTP client (e.g. network/IO).
  ReqwestError(reqwest::Error),
}

impl WanApiError {
  /// Classify a non-success HTTP response into a well-known variant when
  /// possible, falling back to the uncategorized variant.
  pub fn from_status_and_body(status_code: StatusCode, body: String) -> Self {
    let raw_http_body = Some(body.clone());
    match status_code {
      StatusCode::UNAUTHORIZED => Self::Unauthorized { raw_http_body },
      StatusCode::FORBIDDEN => Self::Forbidden { raw_http_body },
      StatusCode::NOT_FOUND => Self::NotFound { raw_http_body },
      StatusCode::TOO_MANY_REQUESTS => Self::RateLimited { raw_http_body },
      StatusCode::BAD_REQUEST => Self::BadRequest { reason: body, raw_http_body },
      _ => Self::UncategorizedBadResponseWithStatusAndBody { status_code, body },
    }
  }
}

impl Error for WanApiError {}

impl Display for WanApiError {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Unauthorized { raw_http_body } => {
        write!(f, "Wan API: Unauthorized (invalid or missing API key){}", fmt_raw_body(raw_http_body))
      },
      Self::Forbidden { raw_http_body } => {
        write!(f, "Wan API: Forbidden{}", fmt_raw_body(raw_http_body))
      },
      Self::NotFound { raw_http_body } => {
        write!(f, "Wan API: Not found{}", fmt_raw_body(raw_http_body))
      },
      Self::RateLimited { raw_http_body } => {
        write!(f, "Wan API: Rate limited{}", fmt_raw_body(raw_http_body))
      },
      Self::BadRequest { reason, raw_http_body } => {
        write!(f, "Wan API: Bad request: {}{}", reason, fmt_raw_body(raw_http_body))
      },
      Self::SerdeResponseParseErrorWithBody(err, body) => {
        write!(f, "Failed to parse Wan API response body: {:?}. Body: {}", err, body)
      },
      Self::UncategorizedBadResponseWithStatusAndBody { status_code, body } => {
        write!(f, "Uncategorized Wan API response: status code {}, body: {}", status_code, body)
      },
      Self::ReqwestError(err) => write!(f, "Reqwest error during Wan API call: {}", err),
    }
  }
}

/// Append the raw HTTP body to a `Display` message when one is present, so
/// `{}`-formatted logs carry the upstream context. Empty when absent.
fn fmt_raw_body(raw_http_body: &Option<String>) -> String {
  match raw_http_body {
    Some(body) if !body.is_empty() => format!(" [raw http body: {}]", body),
    _ => String::new(),
  }
}
