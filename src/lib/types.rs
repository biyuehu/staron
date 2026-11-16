use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;

pub(crate) type AsyncAction<T> =
  Arc<dyn Fn(T) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailReason {
  NotAuthenticated,
  NotStarred,
}

#[derive(Debug, Clone)]
pub struct FailContext {
  pub owner: String,
  pub repo: String,
  pub login: Option<String>,
  pub reason: FailReason,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SuccessContext {
  pub owner: String,
  pub repo: String,
  pub login: String,
  pub starred_at: Option<String>,
  pub cached: bool,
}

#[derive(Clone)]
pub enum FailAction {
  Block(Option<String>),
  Warn(Option<String>),
  Fn(AsyncAction<FailContext>),
}

#[allow(non_snake_case)]
pub fn Block(message: Option<impl Into<String>>) -> FailAction {
  FailAction::Block(message.map(Into::into))
}

#[allow(non_snake_case)]
pub fn Warn(message: impl Into<String>) -> FailAction {
  FailAction::Warn(Some(message.into()))
}

#[allow(non_snake_case)]
pub fn Fn<F, Fut>(run: F) -> FailAction
where
  F: Fn(FailContext) -> Fut + Send + Sync + 'static,
  Fut: Future<Output = ()> + Send + 'static,
{
  FailAction::Fn(Arc::new(move |ctx| Box::pin(run(ctx))))
}

#[derive(Clone)]
pub struct StaronOptions {
  pub client_id: String,
  pub owner: String,
  pub repo: String,
  pub on_fail: Option<FailAction>,
  pub on_success: Option<AsyncAction<SuccessContext>>,
  pub cache_expire_days: Option<u64>,
  pub cache_file_path: Option<PathBuf>,
  pub skip_star_for_owner: Option<bool>,
}

impl StaronOptions {
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

  pub fn on_fail(mut self, action: FailAction) -> Self {
    self.on_fail = Some(action);
    self
  }

  pub fn on_success<F, Fut>(mut self, run: F) -> Self
  where
    F: Fn(SuccessContext) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = ()> + Send + 'static,
  {
    self.on_success = Some(Arc::new(move |ctx| Box::pin(run(ctx))));
    self
  }

  pub fn cache_expire_days(mut self, days: u64) -> Self {
    self.cache_expire_days = Some(days);
    self
  }

  pub fn cache_file_path(mut self, path: impl Into<PathBuf>) -> Self {
    self.cache_file_path = Some(path.into());
    self
  }

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
