use super::types::DeviceCodeResponse;
use reqwest::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE};
use serde::Deserialize;
use serde_json::json;

const DEVICE_CODE_URL: &str = "https://github.com/login/device/code";
const TOKEN_URL: &str = "https://github.com/login/oauth/access_token";

fn client() -> reqwest::Client {
  reqwest::Client::builder()
    .user_agent("staron")
    .build()
    .unwrap_or_default()
}

pub async fn request_device_code(client_id: &str) -> Result<DeviceCodeResponse, String> {
  let res = client()
    .post(DEVICE_CODE_URL)
    .header(CONTENT_TYPE, "application/json")
    .header(ACCEPT, "application/json")
    .json(&json!({ "client_id": client_id }))
    .send()
    .await
    .map_err(|e| e.to_string())?;

  if res.status().is_success() {
    res
      .json::<DeviceCodeResponse>()
      .await
      .map_err(|e| format!("failed to parse device code response: {e}"))
  } else {
    let status = res.status();
    let body = res.text().await.unwrap_or_default();
    Err(format!("device code request failed: {status} {body}"))
  }
}

#[derive(Deserialize)]
struct TokenResponse {
  access_token: Option<String>,
  error: Option<String>,
}

async fn request_token(client_id: &str, device_code: &str) -> Result<String, String> {
  let res = client()
    .post(TOKEN_URL)
    .header(CONTENT_TYPE, "application/json")
    .header(ACCEPT, "application/json")
    .json(&json!({
        "client_id": client_id,
        "device_code": device_code,
        "grant_type": "urn:ietf:params:oauth:grant-type:device_code"
    }))
    .send()
    .await
    .map_err(|e| e.to_string())?;

  let data = res
    .json::<TokenResponse>()
    .await
    .map_err(|e| format!("failed to parse token response: {e}"))?;

  data
    .access_token
    .ok_or_else(|| data.error.unwrap_or_else(|| "unknown error".to_string()))
}

pub async fn poll_for_token(
  client_id: &str,
  device_code: &str,
  interval: u64,
  expires_at: u64,
) -> Result<String, String> {
  loop {
    let now = std::time::SystemTime::now()
      .duration_since(std::time::UNIX_EPOCH)
      .map(|d| d.as_millis() as u64)
      .unwrap_or(0);

    if now > expires_at {
      return Err("device flow expired".to_string());
    }

    match request_token(client_id, device_code).await {
      Ok(token) => return Ok(token),
      Err(err) if err == "authorization_pending" => {
        tokio::time::sleep(std::time::Duration::from_secs(interval)).await;
      }
      Err(err) => return Err(err),
    }
  }
}

#[derive(Deserialize)]
struct UserResponse {
  login: String,
}

pub async fn fetch_login(token: &str) -> Result<String, String> {
  let res = client()
    .get("https://api.github.com/user")
    .header(AUTHORIZATION, format!("Bearer {token}"))
    .send()
    .await
    .map_err(|e| e.to_string())?;

  if res.status().is_success() {
    res
      .json::<UserResponse>()
      .await
      .map(|u| u.login)
      .map_err(|e| format!("failed to parse user response: {e}"))
  } else {
    Err(format!("failed to fetch user: {}", res.status()))
  }
}

pub async fn check_star(token: &str, owner: &str, repo: &str) -> Result<bool, String> {
  let res = client()
    .get(format!(
      "https://api.github.com/user/starred/{owner}/{repo}"
    ))
    .header(AUTHORIZATION, format!("Bearer {token}"))
    .send()
    .await
    .map_err(|e| e.to_string())?;

  match res.status().as_u16() {
    204 => Ok(true),
    404 => Ok(false),
    status => Err(format!("unexpected status {status}")),
  }
}
