//! A client for a user-hosted Wan-AI (DashScope-compatible) gateway.
//!
//! Unlike the other provider clients, Wan is not pinned to a fixed host: the
//! base URL and API key are both supplied by the user.

pub mod api;
pub mod creds;
pub mod error;
