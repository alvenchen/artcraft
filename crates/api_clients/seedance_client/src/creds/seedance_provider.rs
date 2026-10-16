/// Which upstream service serves the Seedance API.
///
/// Seedance is resold by several vendors whose HTTP surfaces differ in more
/// than the host: paths, request body shape, and response fields all vary.
/// The variant is chosen in code by the caller — it is not a user setting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SeedanceProvider {
  /// Volcano Engine (CN) / BytePlus (international) ARK — the native
  /// `/contents/generations/tasks` API.
  VolcanoArk,

  /// ModelVerse's unified async-task API (`/v1/tasks/submit`,
  /// `/v1/tasks/status`), which proxies Seedance alongside other vendors.
  ModelVerse,
}
