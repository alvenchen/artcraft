use crate::creds::wan_provider::WanProvider;

/// The user-configurable base URL of the Wan-AI gateway, paired with the
/// upstream provider that serves it.
///
/// Unlike the other providers, Wan is not pinned to a fixed host: a
/// DashScope-compatible gateway and ModelVerse both resell it under different
/// API shapes. The provider decides which paths [`WanBaseUrl`] builds. Trailing
/// slashes are trimmed so URL joins are predictable.
#[derive(Clone, Debug)]
pub struct WanBaseUrl {
  base_url: String,
  provider: WanProvider,
}

impl WanBaseUrl {
  pub fn new(base_url: impl Into<String>, provider: WanProvider) -> Self {
    let base_url = base_url.into().trim().trim_end_matches('/').to_string();
    Self { base_url, provider }
  }

  pub fn as_str(&self) -> &str {
    &self.base_url
  }

  pub fn provider(&self) -> WanProvider {
    self.provider
  }

  /// The OpenAI-style model listing endpoint.
  ///
  /// - [`WanProvider::DashScope`]: `{base}/models`
  /// - [`WanProvider::ModelVerse`]: `{base}/v1/models`
  pub fn models_url(&self) -> String {
    match self.provider {
      WanProvider::DashScope => format!("{}/models", self.base_url),
      WanProvider::ModelVerse => format!("{}/v1/models", self.base_url),
    }
  }

  /// The task submit endpoint.
  ///
  /// - [`WanProvider::DashScope`]:
  ///   `{base}/api/v1/services/aigc/video-generation/video-synthesis`
  /// - [`WanProvider::ModelVerse`]: `{base}/v1/tasks/submit`
  pub fn video_synthesis_url(&self) -> String {
    match self.provider {
      WanProvider::DashScope => format!("{}/api/v1/services/aigc/video-generation/video-synthesis", self.base_url),
      WanProvider::ModelVerse => format!("{}/v1/tasks/submit", self.base_url),
    }
  }

  /// The task query endpoint.
  ///
  /// - [`WanProvider::DashScope`]: `{base}/api/v1/tasks/{task_id}`
  /// - [`WanProvider::ModelVerse`]: `{base}/v1/tasks/status?task_id={task_id}`
  pub fn task_url(&self, task_id: &str) -> String {
    match self.provider {
      WanProvider::DashScope => format!("{}/api/v1/tasks/{}", self.base_url, task_id),
      WanProvider::ModelVerse => format!("{}/v1/tasks/status?task_id={}", self.base_url, task_id),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn dashscope_urls_use_the_dashscope_paths() {
    let base_url = WanBaseUrl::new("https://gateway.example.com/", WanProvider::DashScope);

    assert_eq!(base_url.as_str(), "https://gateway.example.com");
    assert_eq!(base_url.models_url(), "https://gateway.example.com/models");
    assert_eq!(base_url.video_synthesis_url(), "https://gateway.example.com/api/v1/services/aigc/video-generation/video-synthesis");
    assert_eq!(base_url.task_url("task_123"), "https://gateway.example.com/api/v1/tasks/task_123");
  }

  #[test]
  fn modelverse_urls_use_the_unified_task_paths() {
    let base_url = WanBaseUrl::new("https://api.modelverse.cn/", WanProvider::ModelVerse);

    assert_eq!(base_url.provider(), WanProvider::ModelVerse);
    assert_eq!(base_url.models_url(), "https://api.modelverse.cn/v1/models");
    assert_eq!(base_url.video_synthesis_url(), "https://api.modelverse.cn/v1/tasks/submit");
    assert_eq!(base_url.task_url("task_123"), "https://api.modelverse.cn/v1/tasks/status?task_id=task_123");
  }
}
