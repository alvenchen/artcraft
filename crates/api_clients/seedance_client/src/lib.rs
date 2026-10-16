//! A client for ByteDance's Seedance video generation via the Volcano Engine
//! ARK API. Auth is an API key; the base URL is supplied by the user (Volcano
//! Engine and BytePlus expose the same API under different hosts).

pub mod api;
pub mod creds;
pub mod error;
