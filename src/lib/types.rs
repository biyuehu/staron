use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;

pub(crate) type AsyncAction<T> =
  Arc<dyn Fn(T) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;

/// Reason for star gate verification failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailReason {
  /// GitHub OAuth Device Flow could not be completed or credentials were invalid.
  NotAuthenticated,
  /// Authenticated user has not starred the target repository.
  NotStarred,
}

/// Diagnostic context provided to failure handlers.
#[derive(Debug, Clone)]
pub struct FailContext {
  /// Target repository owner.
  pub owner: String,
  /// Target repository name.
  pub repo: String,
  /// Authenticated GitHub username, if authentication was successful.
  pub login: Option<String>,
  /// Specific reason why verification failed.
  pub reason: FailReason,
}

/// Context provided to successful verification handlers.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SuccessContext {
  /// Target repository owner.
  pub owner: String,
  /// Target repository name.
  pub repo: String,
  /// Authenticated GitHub username.
  pub login: String,
  /// Timestamp of when the star was given, if provided by the API.
  pub starred_at: Option<String>,
  /// Whether verification was satisfied from the local cache.
  pub cached: bool,
}

/// Action to execute when star gate verification fails.
#[derive(Clone)]
pub enum FailAction {
  /// Print error message to `stderr` and terminate the process with exit code 1.
  Block(Option<String>),
  /// Print warning message to `stderr` and allow process execution to continue.
  Warn(Option<String>),
  /// Execute a custom asynchronous failure callback.
  Fn(AsyncAction<FailContext>),
}

/// Constructs a `FailAction::Block` that halts execution on verification failure.
///
/// If `message` is `None`, a default prompt asking the user to star the repository is shown.
#[allow(non_snake_case)]
pub fn Block(message: Option<impl Into<String>>) -> FailAction {
  FailAction::Block(message.map(Into::into))
}

/// Constructs a `FailAction::Warn` that prints a warning on verification failure without exiting.
#[allow(non_snake_case)]
pub fn Warn(message: impl Into<String>) -> FailAction {
  FailAction::Warn(Some(message.into()))
}

/// Constructs a `FailAction::Fn` wrapping an asynchronous failure callback.
#[allow(non_snake_case)]
pub fn Fn<F, Fut>(run: F) -> FailAction
where
  F: Fn(FailContext) -> Fut + Send + Sync + 'static,
  Fut: Future<Output = ()> + Send + 'static,
{
  FailAction::Fn(Arc::new(move |ctx| Box::pin(run(ctx))))
}

/// Configuration options for star gate verification.
#[derive(Clone)]
pub struct StaronOptions {
  /// GitHub OAuth App client ID.
  pub client_id: String,
  /// Target repository owner.
  pub owner: String,
  /// Target repository name.
  pub repo: String,
  /// Action taken when verification fails. Defaults to `FailAction::Block(None)`.
  pub on_fail: Option<FailAction>,
  /// Asynchronous callback executed when verification succeeds.
  pub on_success: Option<AsyncAction<SuccessContext>>,
  /// Number of days before local cache validity expires. Defaults to 3.
  pub cache_expire_days: Option<u64>,
  /// Custom path for the cache file. Defaults to `~/.config/staron/<owner>-<repo>.json`.
  pub cache_file_path: Option<PathBuf>,
  /// Whether to automatically bypass the star requirement if the user is the repository owner.
  pub skip_star_for_owner: Option<bool>,
}

impl StaronOptions {
  /// Initializes options with mandatory GitHub application and repository coordinates.
  pub fn new(
    client_id: impl Into<String>,
    owner: impl Into<String>,
    repo: impl Into<String>,
  ) -> Self {
    Self {
      client_id: client_id.into(),
      owner: owner.into(),
      repo: repo.into(),
      on_fail: None,
      on_success: None,
      cache_expire_days: None,
      cache_file_path: None,
      skip_star_for_owner: None,
    }
  }

  /// Sets the failure handling action.
  pub fn on_fail(mut self, action: FailAction) -> Self {
    self.on_fail = Some(action);
    self
  }

  /// Sets an asynchronous callback for successful verification.
  pub fn on_success<F, Fut>(mut self, run: F) -> Self
  where
    F: Fn(SuccessContext) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = ()> + Send + 'static,
  {
    self.on_success = Some(Arc::new(move |ctx| Box::pin(run(ctx))));
    self
  }

  /// Sets the cache validity duration in days.
  pub fn cache_expire_days(mut self, days: u64) -> Self {
    self.cache_expire_days = Some(days);
    self
  }

  /// Sets a custom filesystem path for the star cache file.
  pub fn cache_file_path(mut self, path: impl Into<PathBuf>) -> Self {
    self.cache_file_path = Some(path.into());
    self
  }

  /// Sets whether the star requirement is bypassed when the logged-in user owns the repository.
  pub fn skip_star_for_owner(mut self, skip: bool) -> Self {
    self.skip_star_for_owner = Some(skip);
    self
  }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub(crate) struct CacheData {
  pub login: String,
  pub token: String,
  pub starred: bool,
  #[serde(rename = "checkedAt")]
  pub checked_at: u64,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub(crate) struct DeviceCodeResponse {
  pub device_code: String,
  pub user_code: String,
  pub verification_uri: String,
  pub expires_in: u64,
  pub interval: u64,
}

#[derive(Debug, Clone)]
pub(crate) struct GateFailure {
  pub reason: FailReason,
  pub message: String,
}

pub(crate) type GateResult = Result<SuccessContext, GateFailure>;

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn block_produces_a_block_action_with_optional_message() {
    match Block(Some("stop")) {
      FailAction::Block(msg) => assert_eq!(msg.as_deref(), Some("stop")),
      _ => panic!("expected FailAction::Block"),
    }
    match Block(None::<String>) {
      FailAction::Block(msg) => assert_eq!(msg, None),
      _ => panic!("expected FailAction::Block"),
    }
  }

  #[test]
  fn warn_produces_a_warn_action_with_optional_message() {
    match Warn("careful") {
      FailAction::Warn(msg) => assert_eq!(msg.as_deref(), Some("careful")),
      _ => panic!("expected FailAction::Warn"),
    }
  }

  #[tokio::test]
  async fn fn_produces_a_fn_action_wrapping_the_given_callback() {
    let action = Fn(|ctx| async move {
      assert_eq!(ctx.owner, "test_owner");
    });
    match action {
      FailAction::Fn(run) => {
        run(FailContext {
          owner: "test_owner".to_string(),
          repo: "test_repo".to_string(),
          login: None,
          reason: FailReason::NotStarred,
        })
        .await;
      }
      _ => panic!("expected FailAction::Fn"),
    }
  }
}
