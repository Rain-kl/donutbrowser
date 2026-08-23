use crate::profile::types::get_host_os;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

pub const BROWSER_ID: &str = "fingerprint_chromium";
pub const DEFAULT_BROWSER: &str = BROWSER_ID;
pub const PINNED_VERSION: &str = "144.0.7559.132";
const WAYFERN_DEFAULT_BACKUP: &str = "_donut_wayfern_default_backup";
const MIGRATION_MARKER: &str = ".donut-fingerprint-profile-migrated";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FingerprintChromiumConfig {
  pub seed: u32,
  #[serde(default = "get_host_os")]
  pub platform: String,
  #[serde(default)]
  pub platform_version: Option<String>,
  #[serde(default = "default_brand")]
  pub brand: String,
  #[serde(default)]
  pub brand_version: Option<String>,
  #[serde(default)]
  pub hardware_concurrency: Option<u8>,
  #[serde(default)]
  pub timezone: Option<String>,
  #[serde(default)]
  pub languages: Vec<String>,
  #[serde(default = "default_true")]
  pub disable_non_proxied_udp: bool,
  #[serde(default)]
  pub disabled_spoofing: Vec<String>,
  #[serde(default)]
  pub geoip: Option<serde_json::Value>,
  #[serde(default)]
  pub block_images: Option<bool>,
  #[serde(default)]
  pub block_webgl: Option<bool>,
  #[serde(default)]
  pub geo_proxy_signature: Option<String>,
}

impl Default for FingerprintChromiumConfig {
  fn default() -> Self {
    Self {
      seed: 1,
      platform: get_host_os(),
      platform_version: None,
      brand: default_brand(),
      brand_version: None,
      hardware_concurrency: None,
      timezone: None,
      languages: Vec::new(),
      disable_non_proxied_udp: true,
      disabled_spoofing: Vec::new(),
      geoip: None,
      block_images: None,
      block_webgl: None,
      geo_proxy_signature: None,
    }
  }
}

impl FingerprintChromiumConfig {
  pub fn for_profile(profile_id: &uuid::Uuid) -> Self {
    Self {
      seed: stable_seed(profile_id),
      ..Self::default()
    }
  }

  pub fn launch_args(&self) -> Vec<String> {
    let mut args = vec![
      format!("--fingerprint={}", self.seed.max(1)),
      format!("--fingerprint-platform={}", self.platform),
      format!("--fingerprint-brand={}", self.brand),
    ];

    if let Some(value) = non_empty(self.platform_version.as_deref()) {
      args.push(format!("--fingerprint-platform-version={value}"));
    }
    if let Some(value) = non_empty(self.brand_version.as_deref()) {
      args.push(format!("--fingerprint-brand-version={value}"));
    }
    if let Some(value) = self.hardware_concurrency.filter(|value| *value > 0) {
      args.push(format!("--fingerprint-hardware-concurrency={value}"));
    }
    if let Some(value) = non_empty(self.timezone.as_deref()) {
      args.push(format!("--timezone={value}"));
    }
    if let Some(language) = self
      .languages
      .first()
      .and_then(|value| non_empty(Some(value)))
    {
      args.push(format!("--lang={language}"));
    }
    if !self.languages.is_empty() {
      args.push(format!("--accept-lang={}", self.languages.join(",")));
    }
    if self.disable_non_proxied_udp {
      args.push("--disable-non-proxied-udp".to_string());
    }
    if !self.disabled_spoofing.is_empty() {
      args.push(format!(
        "--disable-spoofing={}",
        self.disabled_spoofing.join(",")
      ));
    }
    if self.block_images == Some(true) {
      args.push("--blink-settings=imagesEnabled=false".to_string());
    }
    if self.block_webgl == Some(true) {
      args.push("--disable-webgl".to_string());
    }

    args
  }

  pub async fn refresh_geolocation(&mut self, proxy: Option<&str>) -> bool {
    if matches!(self.geoip, Some(serde_json::Value::Bool(false))) {
      return false;
    }

    let ip = match self.geoip.as_ref() {
      Some(serde_json::Value::String(ip)) if !ip.trim().is_empty() => ip.clone(),
      _ => match crate::ip_utils::fetch_public_ip(proxy).await {
        Ok(ip) => ip,
        Err(error) => {
          log::warn!("Failed to resolve fingerprint-chromium public IP: {error}");
          return false;
        }
      },
    };

    match crate::geolocation::get_geolocation(&ip) {
      Ok(geo) => {
        let locale = geo.locale.as_string();
        self.timezone = Some(geo.timezone);
        self.languages = vec![locale, geo.locale.language];
        true
      }
      Err(error) => {
        log::warn!("Failed to resolve fingerprint-chromium geolocation for {ip}: {error}");
        false
      }
    }
  }
}

pub fn stable_seed(profile_id: &uuid::Uuid) -> u32 {
  let hash = blake3::hash(profile_id.as_bytes());
  let bytes = hash.as_bytes();
  u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]).max(1)
}

fn non_empty(value: Option<&str>) -> Option<&str> {
  value.map(str::trim).filter(|value| !value.is_empty())
}

fn default_brand() -> String {
  "Chrome".to_string()
}

fn default_true() -> bool {
  true
}

pub fn migrate_wayfern_user_data(
  profile_path: &Path,
) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
  let marker = profile_path.join(MIGRATION_MARKER);
  if marker.exists() {
    return Ok(false);
  }

  fs::create_dir_all(profile_path)?;
  let default_dir = profile_path.join("Default");
  let test_backup = profile_path.join("_Default.wayfern-backup");
  let backup_dir = profile_path.join(WAYFERN_DEFAULT_BACKUP);

  if !backup_dir.exists() {
    let source = if test_backup.exists() {
      test_backup
    } else {
      default_dir.clone()
    };
    if source.exists() {
      fs::rename(&source, &backup_dir)?;
    }
  }
  fs::create_dir_all(&default_dir)?;

  if backup_dir.exists() {
    const PRESERVED_ENTRIES: &[&str] = &[
      "Bookmarks",
      "Bookmarks.bak",
      "Favicons",
      "Favicons-journal",
      "History",
      "History-journal",
      "IndexedDB",
      "Local Extension Settings",
      "Local Storage",
      "Login Data",
      "Login Data For Account",
      "Network",
      "Service Worker",
      "Storage",
      "Sync Extension Settings",
      "Web Data",
      "Web Data-journal",
      "WebStorage",
    ];
    for entry in PRESERVED_ENTRIES {
      let source = backup_dir.join(entry);
      if source.exists() {
        copy_path(&source, &default_dir.join(entry))?;
      }
    }
  }

  fs::write(
    &marker,
    format!("migrated_from=wayfern\nbrowser={BROWSER_ID}\nversion={PINNED_VERSION}\n"),
  )?;
  Ok(true)
}

fn copy_path(
  source: &Path,
  destination: &Path,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
  if source.is_dir() {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
      let entry = entry?;
      copy_path(&entry.path(), &destination.join(entry.file_name()))?;
    }
  } else {
    if let Some(parent) = destination.parent() {
      fs::create_dir_all(parent)?;
    }
    fs::copy(source, destination)?;
  }
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::{migrate_wayfern_user_data, stable_seed, FingerprintChromiumConfig};
  use std::fs;

  #[test]
  fn profile_seed_is_stable_and_non_zero() {
    let id = uuid::Uuid::parse_str("9b93207c-e8ca-497a-91ea-8c6bb8bd968a").unwrap();
    assert_eq!(stable_seed(&id), stable_seed(&id));
    assert_ne!(stable_seed(&id), 0);
  }

  #[test]
  fn launch_args_include_stable_identity() {
    let config = FingerprintChromiumConfig {
      seed: 42,
      platform: "windows".to_string(),
      timezone: Some("Asia/Shanghai".to_string()),
      languages: vec!["zh-CN".to_string(), "zh".to_string()],
      ..FingerprintChromiumConfig::default()
    };
    let args = config.launch_args();
    assert!(args.contains(&"--fingerprint=42".to_string()));
    assert!(args.contains(&"--fingerprint-platform=windows".to_string()));
    assert!(args.contains(&"--timezone=Asia/Shanghai".to_string()));
    assert!(args.contains(&"--accept-lang=zh-CN,zh".to_string()));
  }

  #[test]
  fn migration_preserves_identity_data_and_skips_version_sensitive_state() {
    let temp = tempfile::tempdir().unwrap();
    let default = temp.path().join("Default");
    fs::create_dir_all(default.join("Network")).unwrap();
    fs::create_dir_all(default.join("Code Cache")).unwrap();
    fs::write(default.join("Network").join("Cookies"), b"cookies").unwrap();
    fs::write(default.join("History"), b"history").unwrap();
    fs::write(
      default.join("Preferences"),
      b"{\"created_by_version\":\"149\"}",
    )
    .unwrap();
    fs::write(default.join("Code Cache").join("index"), b"cache").unwrap();

    assert!(migrate_wayfern_user_data(temp.path()).unwrap());
    assert_eq!(
      fs::read(default.join("Network").join("Cookies")).unwrap(),
      b"cookies"
    );
    assert_eq!(fs::read(default.join("History")).unwrap(), b"history");
    assert!(!default.join("Preferences").exists());
    assert!(!default.join("Code Cache").exists());
    assert!(temp.path().join("_donut_wayfern_default_backup").exists());
    assert!(!migrate_wayfern_user_data(temp.path()).unwrap());
  }
}
