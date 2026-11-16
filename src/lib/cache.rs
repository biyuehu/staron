use super::types::CacheData;
use std::fs;
use std::path::{Path, PathBuf};

pub fn default_cache_path(owner: &str, repo: &str) -> PathBuf {
  dirs::home_dir()
    .unwrap_or_else(|| PathBuf::from("."))
    .join(".config")
    .join("staron")
    .join(format!("{owner}-{repo}.json"))
}

pub fn read_cache(path: &Path) -> Result<CacheData, String> {
  if !path.exists() {
    return Err("cache not found".to_string());
  }
  fs::read_to_string(path)
    .map_err(|_| "cache corrupted".to_string())
    .and_then(|content| serde_json::from_str(&content).map_err(|_| "cache corrupted".to_string()))
}

pub fn write_cache(path: &Path, data: &CacheData) -> Result<(), String> {
  if let Some(parent) = path.parent() {
    fs::create_dir_all(parent).map_err(|_| "failed to write cache".to_string())?;
  }
  let content = serde_json::to_string(data).map_err(|_| "failed to write cache".to_string())?;
  #[cfg(unix)]
  {
    use std::os::unix::fs::OpenOptionsExt;
    fs::OpenOptions::new()
      .write(true)
      .create(true)
      .truncate(true)
      .mode(0o600)
      .open(path)
      .and_then(|mut f| std::io::Write::write_all(&mut f, content.as_bytes()))
      .map_err(|_| "failed to write cache".to_string())
  }
  #[cfg(not(unix))]
  {
    fs::write(path, content).map_err(|_| "failed to write cache".to_string())
  }
}

pub fn is_cache_fresh(checked_at: u64, expire_days: u64) -> bool {
  let now = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .map(|d| d.as_millis() as u64)
    .unwrap_or(0);
  now.saturating_sub(checked_at) < expire_days * 86_400_000
}

#[cfg(test)]
mod tests {
  use super::*;
  use tempfile::tempdir;

  #[test]
  fn returns_err_when_cache_file_does_not_exist() {
    let dir = tempdir().unwrap();
    assert!(read_cache(&dir.path().join("missing.json")).is_err());
  }

  #[test]
  fn writes_and_reads_back_the_same_data() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("cache.json");
    let now = std::time::SystemTime::now()
      .duration_since(std::time::UNIX_EPOCH)
      .unwrap()
      .as_millis() as u64;
    let data = CacheData {
      login: "biyuehu".to_string(),
      token: "tok".to_string(),
      starred: true,
      checked_at: now,
    };
    assert!(write_cache(&path, &data).is_ok());
    let result = read_cache(&path);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), data);
  }

  #[test]
  fn returns_err_when_cache_file_is_corrupted() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("bad.json");
    let data = CacheData {
      login: "x".to_string(),
      token: "x".to_string(),
      starred: false,
      checked_at: 0,
    };
    assert!(write_cache(&path, &data).is_ok());
    fs::write(&path, "not json").unwrap();
    assert!(read_cache(&path).is_err());
  }

  #[test]
  fn treats_a_recent_timestamp_as_fresh() {
    let now = std::time::SystemTime::now()
      .duration_since(std::time::UNIX_EPOCH)
      .unwrap()
      .as_millis() as u64;
    assert!(is_cache_fresh(now, 7));
  }

  #[test]
  fn treats_an_old_timestamp_as_stale() {
    let now = std::time::SystemTime::now()
      .duration_since(std::time::UNIX_EPOCH)
      .unwrap()
      .as_millis() as u64;
    assert!(!is_cache_fresh(now.saturating_sub(10 * 86_400_000), 7));
  }
}
