//! Functional GitHub star gate for CLI tools.
//!
//! Provides GitHub OAuth Device Flow authentication and star verification
//! with local token caching and customizable failure actions.

mod lib {
  pub mod cache;
  pub mod github;
  pub mod types;
}

pub use lib::types::{
  Block, FailAction, FailContext, FailReason, Fn, StaronOptions, SuccessContext, Warn,
};

use lib::cache::{default_cache_path, is_cache_fresh, read_cache, write_cache};
use lib::github::{check_star, fetch_login, poll_for_token, request_device_code};
use lib::types::{self, GateFailure, GateResult};
use std::future::Future;
use std::pin::Pin;

fn default_message(ctx: &FailContext) -> String {
  match ctx.reason {
    FailReason::NotAuthenticated => {
      "GitHub authentication failed or was not completed. Please try again.".to_string()
    }
    FailReason::NotStarred => format!(
      "Please star {}/{} on GitHub to use this tool.",
      ctx.owner, ctx.repo
    ),
  }
}

async fn run_fail_action(options: &StaronOptions, ctx: FailContext, detail: &str) {
  let action = options
    .on_fail
    .as_ref()
    .cloned()
    .unwrap_or(FailAction::Block(None));
  match action {
    FailAction::Fn(run) => run(ctx).await,
    FailAction::Warn(msg) => {
      eprintln!("{}", msg.unwrap_or_else(|| default_message(&ctx)));
      eprintln!("detail: {detail}");
    }
    FailAction::Block(msg) => {
      eprintln!("{}", msg.unwrap_or_else(|| default_message(&ctx)));
      eprintln!("detail: {detail}");
      std::process::exit(1);
    }
  }
}

const MAX_AUTH_RETRIES: u32 = 3;

fn now_ms() -> u64 {
  std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .map(|d| d.as_millis() as u64)
    .unwrap_or(0)
}

fn authenticate<'a>(
  options: &'a StaronOptions,
  why: &'a str,
  attempt: u32,
) -> Pin<Box<dyn Future<Output = GateResult> + Send + 'a>> {
  Box::pin(async move {
    if attempt == 0 {
      println!("GitHub login required: {why}");
    }
    let device_res = match request_device_code(&options.client_id).await {
      Ok(res) => res,
      Err(e) => {
        return if attempt < MAX_AUTH_RETRIES {
          authenticate(options, why, attempt + 1).await
        } else {
          Err(GateFailure {
            reason: FailReason::NotAuthenticated,
            message: e,
          })
        };
      }
    };

    println!(
      "Please visit {} and enter code: {}",
      device_res.verification_uri, device_res.user_code
    );

    let token = match poll_for_token(
      &options.client_id,
      &device_res.device_code,
      device_res.interval,
      now_ms() + device_res.expires_in * 1000,
    )
    .await
    {
      Ok(tok) => tok,
      Err(e) => {
        return if attempt < MAX_AUTH_RETRIES {
          authenticate(options, why, attempt + 1).await
        } else {
          Err(GateFailure {
            reason: FailReason::NotAuthenticated,
            message: e,
          })
        };
      }
    };

    let login = match fetch_login(&token).await {
      Ok(log) => log,
      Err(e) => {
        return if attempt < MAX_AUTH_RETRIES {
          authenticate(options, why, attempt + 1).await
        } else {
          Err(GateFailure {
            reason: FailReason::NotAuthenticated,
            message: e,
          })
        };
      }
    };

    let skip_star = options.skip_star_for_owner.unwrap_or(false) && login == options.owner;
    let starred = if skip_star {
      true
    } else {
      match check_star(&token, &options.owner, &options.repo).await {
        Ok(s) => s,
        Err(e) => {
          return if attempt < MAX_AUTH_RETRIES {
            authenticate(options, why, attempt + 1).await
          } else {
            Err(GateFailure {
              reason: FailReason::NotAuthenticated,
              message: e,
            })
          };
        }
      }
    };

    let cache_path = options
      .cache_file_path
      .clone()
      .unwrap_or_else(|| default_cache_path(&options.owner, &options.repo));
    let _ = write_cache(
      &cache_path,
      &types::CacheData {
        login: login.clone(),
        token,
        starred,
        checked_at: now_ms(),
      },
    );

    if starred {
      Ok(SuccessContext {
        login,
        starred_at: None,
        cached: false,
        owner: options.owner.clone(),
        repo: options.repo.clone(),
      })
    } else {
      Err(GateFailure {
        reason: FailReason::NotStarred,
        message: "user has not starred the repository".to_string(),
      })
    }
  })
}

/// Enforces a GitHub star verification gate.
///
/// Verifies whether the local user has starred the target repository. If a valid,
/// unexpired cache entry is found, verification succeeds immediately without network
/// requests. Otherwise, re-checks star status via the saved token or triggers an
/// interactive OAuth Device Flow login.
///
/// # Side Effects
/// - Reads from and writes to the local cache directory (`~/.config/staron/`).
/// - Sends HTTP requests to GitHub OAuth and REST APIs.
/// - Prompts interactive login steps to `stdout` when authentication is needed.
/// - Invokes `on_success` callback on success, or executes `on_fail` action (which
///   may terminate the process via `std::process::exit(1)`) on failure.
pub async fn gate(options: StaronOptions) {
  let cache_path = options
    .cache_file_path
    .clone()
    .unwrap_or_else(|| default_cache_path(&options.owner, &options.repo));
  let expire_days = options.cache_expire_days.unwrap_or(3);
  let cache_res = read_cache(&cache_path);

  let result: GateResult = match &cache_res {
    Ok(cache) if cache.starred && is_cache_fresh(cache.checked_at, expire_days) => {
      Ok(SuccessContext {
        login: cache.login.clone(),
        starred_at: None,
        cached: true,
        owner: options.owner.clone(),
        repo: options.repo.clone(),
      })
    }
    Ok(cache) => {
      let skip_star = options.skip_star_for_owner.unwrap_or(false) && cache.login == options.owner;
      let star_res = if skip_star {
        Ok(true)
      } else {
        check_star(&cache.token, &options.owner, &options.repo).await
      };

      match star_res {
        Err(err) => {
          let expired = !is_cache_fresh(cache.checked_at, expire_days);
          authenticate(
            &options,
            &format!(
              "{}, saved token check failed ({err})",
              if expired {
                "cache expired"
              } else {
                "not starred cache recheck"
              }
            ),
            0,
          )
          .await
        }
        Ok(starred) => {
          let _ = write_cache(
            &cache_path,
            &types::CacheData {
              login: cache.login.clone(),
              token: cache.token.clone(),
              starred,
              checked_at: now_ms(),
            },
          );
          if starred {
            Ok(SuccessContext {
              login: cache.login.clone(),
              starred_at: None,
              cached: false,
              owner: options.owner.clone(),
              repo: options.repo.clone(),
            })
          } else {
            Err(GateFailure {
              reason: FailReason::NotStarred,
              message: "user has not starred the repository".to_string(),
            })
          }
        }
      }
    }
    Err(err) => {
      let why = if err == "cache not found" {
        "first use, no saved token".to_string()
      } else {
        format!("saved token unusable ({err})")
      };
      authenticate(&options, &why, 0).await
    }
  };

  match result {
    Ok(success) => {
      if let Some(on_success) = &options.on_success {
        on_success(success).await;
      }
    }
    Err(failure) => {
      run_fail_action(
        &options,
        FailContext {
          login: cache_res.ok().map(|c| c.login),
          owner: options.owner.clone(),
          repo: options.repo.clone(),
          reason: failure.reason,
        },
        &failure.message,
      )
      .await;
    }
  }
}
