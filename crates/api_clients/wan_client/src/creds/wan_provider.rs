/// Which upstream service serves the Wan-AI API.
///
/// Wan is resold by several gateways whose HTTP surfaces differ in more than
/// the host: paths, request body shape, and response fields all vary. The
/// variant is chosen in code by the caller — it is not a user setting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WanProvider {
  /// A DashScope-compatible gateway — the native
  /// `/api/v1/services/aigc/video-generation/video-synthesis` API.
  DashScope,

  /// ModelVerse's unified async-task API (`/v1/tasks/submit`,
  /// `/v1/tasks/status`), which proxies Wan alongside other vendors.
  ModelVerse,
}
