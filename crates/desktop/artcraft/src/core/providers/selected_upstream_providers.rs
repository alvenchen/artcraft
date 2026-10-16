//! Which upstream serves each resold provider. Hardcoded for now — this is not
//! a user setting: pending a real provider picker, the app talks to ModelVerse's
//! unified task API and the user only supplies the base URL and API key.
//!
//! Submit and poll must agree on the provider, so both read these constants
//! rather than spelling the variant out at each call site.

use seedance_client::creds::seedance_provider::SeedanceProvider;
use wan_client::creds::wan_provider::WanProvider;

pub const SEEDANCE_PROVIDER: SeedanceProvider = SeedanceProvider::ModelVerse;
pub const WAN_PROVIDER: WanProvider = WanProvider::ModelVerse;
