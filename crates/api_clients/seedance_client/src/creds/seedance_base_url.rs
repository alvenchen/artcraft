use crate::creds::seedance_provider::SeedanceProvider;

/// The user-configurable base URL of the Seedance endpoint, paired with the
/// upstream provider that serves it.
///
/// Unlike the other providers, Seedance is not pinned to a fixed host: the
/// Volcano Engine (CN) and BytePlus (international) deployments expose the same
/// API under different hosts, and ModelVerse resells it under a different API
/// shape entirely. The provider decides which paths [`SeedanceBaseUrl`] builds.
/// Trailing slashes are trimmed so URL joins are predictable.
#[derive(Clone, Debug)]
pub struct SeedanceBaseUrl {
  base_url: String,
  provider: SeedanceProvider,
}

impl SeedanceBaseUrl {
  pub fn new(base_url: impl Into<String>, provider: SeedanceProvider) -> Self {
    let base_url = base_url.into().trim().trim_end_matches('/').to_string();
    Self { base_url, provider }
  }

  pub fn as_str(&self) -> &str {
    &self.base_url
  }

  pub fn provider(&self) -> SeedanceProvider {
    self.provider
  }

  /// The task submit endpoint.
  ///
  /// - [`SeedanceProvider::VolcanoArk`]: `{base}/contents/generations/tasks`
  /// - [`SeedanceProvider::ModelVerse`]: `{base}/v1/tasks/submit`
  pub fn submit_url(&self) -> String {
    match self.provider {
      SeedanceProvider::VolcanoArk => format!("{}/contents/generations/tasks", self.base_url),
      SeedanceProvider::ModelVerse => format!("{}/v1/tasks/submit", self.base_url),
    }
  }

  /// The task query endpoint.
  ///
  /// - [`SeedanceProvider::VolcanoArk`]: `{base}/contents/generations/tasks/{task_id}`
  /// - [`SeedanceProvider::ModelVerse`]: `{base}/v1/tasks/status?task_id={task_id}`
  pub fn task_url(&self, task_id: &str) -> String {
    match self.provider {
      SeedanceProvider::VolcanoArk => format!("{}/contents/generations/tasks/{}", self.base_url, task_id),
      SeedanceProvider::ModelVerse => format!("{}/v1/tasks/status?task_id={}", self.base_url, task_id),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn volcano_ark_urls_use_the_ark_paths() {
    let base_url = SeedanceBaseUrl::new("https://ark.cn-beijing.volces.com/api/v3/", SeedanceProvider::VolcanoArk);

    assert_eq!(base_url.as_str(), "https://ark.cn-beijing.volces.com/api/v3");
    assert_eq!(base_url.submit_url(), "https://ark.cn-beijing.volces.com/api/v3/contents/generations/tasks");
    assert_eq!(base_url.task_url("task_123"), "https://ark.cn-beijing.volces.com/api/v3/contents/generations/tasks/task_123");
  }

  #[test]
  fn modelverse_urls_use_the_unified_task_paths() {
    let base_url = SeedanceBaseUrl::new("https://api.modelverse.cn/", SeedanceProvider::ModelVerse);

    assert_eq!(base_url.provider(), SeedanceProvider::ModelVerse);
    assert_eq!(base_url.submit_url(), "https://api.modelverse.cn/v1/tasks/submit");
    assert_eq!(base_url.task_url("task_123"), "https://api.modelverse.cn/v1/tasks/status?task_id=task_123");
  }
}
