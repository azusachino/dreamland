use anyhow::bail;
use dreamland_core::{ContentPolicy, MediaVariant, NetworkPolicy, ProxyMode};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub download_path: PathBuf,
    #[serde(default = "default_images_per_page")]
    pub images_per_page: usize,
    #[serde(default)]
    pub content_policy: ContentPolicy,
    #[serde(default = "default_download_variant")]
    pub download_variant: MediaVariant,
    #[serde(default)]
    pub network: NetworkPolicy,
    #[serde(default = "default_site_configs")]
    pub sites: BTreeMap<String, SiteConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SiteConfig {
    #[serde(default = "default_site_enabled")]
    pub enabled: bool,
    #[serde(default = "default_extension_version")]
    pub extension_version: u32,
    #[serde(default)]
    pub extension: BTreeMap<String, toml::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProxyDetection {
    pub detected: bool,
    pub source: Option<String>,
    pub endpoint: Option<String>,
}

fn default_images_per_page() -> usize {
    20
}

fn default_download_variant() -> MediaVariant {
    MediaVariant::Full
}

fn default_site_enabled() -> bool {
    true
}

fn default_extension_version() -> u32 {
    1
}

fn default_site_config(enabled: bool) -> SiteConfig {
    SiteConfig {
        enabled,
        extension_version: 1,
        extension: BTreeMap::new(),
    }
}

fn default_site_configs() -> BTreeMap<String, SiteConfig> {
    BTreeMap::from([
        ("konachan".to_owned(), default_site_config(true)),
        ("pixiv".to_owned(), default_site_config(false)),
        ("twitter".to_owned(), default_site_config(false)),
        ("yandere".to_owned(), default_site_config(true)),
    ])
}

fn default_download_path() -> PathBuf {
    if let Some(dir) = std::env::var_os("DREAMLAND_DOWNLOAD_DIR") {
        return PathBuf::from(dir);
    }
    dirs::download_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("dreamland_images")
}

impl AppConfig {
    pub fn from_user_input(download_path: String) -> anyhow::Result<Self> {
        let download_path = expand_home_path(download_path)?;

        Ok(Self {
            download_path,
            images_per_page: default_images_per_page(),
            content_policy: ContentPolicy::default(),
            download_variant: default_download_variant(),
            network: NetworkPolicy::default(),
            sites: default_site_configs(),
        })
    }

    /// Load TOML settings or build the app-wide defaults.
    /// Site endpoints remain owned by the adapter; `[sites.<id>]` stores only
    /// non-secret user configuration and versioned extension values.
    pub fn load_or_default() -> anyhow::Result<Self> {
        let config_path = Self::config_dir().join("config.toml");
        if config_path.exists() {
            let mut config: Self = toml::from_str(&std::fs::read_to_string(config_path)?)?;
            config.download_path =
                expand_home_path(config.download_path.to_string_lossy().into_owned())?;
            config.validate()?;
            Ok(config)
        } else {
            Ok(Self {
                download_path: default_download_path(),
                images_per_page: default_images_per_page(),
                content_policy: ContentPolicy::default(),
                download_variant: default_download_variant(),
                network: NetworkPolicy::default(),
                sites: default_site_configs(),
            })
        }
    }

    pub fn save(&self) -> anyhow::Result<()> {
        self.validate()?;
        let config_dir = Self::config_dir();
        std::fs::create_dir_all(&config_dir)?;
        std::fs::write(
            config_dir.join("config.toml"),
            toml::to_string_pretty(self)?,
        )?;
        Ok(())
    }

    pub fn enabled_site_ids(&self) -> impl Iterator<Item = &str> {
        self.sites
            .iter()
            .filter(|(_, site)| site.enabled)
            .map(|(site_id, _)| site_id.as_str())
    }

    fn validate(&self) -> anyhow::Result<()> {
        if self.download_path.as_os_str().is_empty() {
            bail!("download path cannot be empty");
        }
        for (site_id, site) in &self.sites {
            if site_id.trim().is_empty() {
                bail!("site config id cannot be empty");
            }
            if site.extension_version == 0 {
                bail!("site config extension version must be positive: {site_id}");
            }
        }
        validate_network_policy(&self.network)
    }

    fn config_dir() -> PathBuf {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(dirs::config_dir)
            .unwrap_or_else(|| PathBuf::from("."))
            .join("dreamland")
    }
}

pub fn detect_proxy() -> ProxyDetection {
    for name in [
        "HTTPS_PROXY",
        "https_proxy",
        "HTTP_PROXY",
        "http_proxy",
        "ALL_PROXY",
        "all_proxy",
    ] {
        if let Ok(value) = std::env::var(name) {
            if let Some(endpoint) = redact_proxy_url(&value) {
                return ProxyDetection {
                    detected: true,
                    source: Some(format!("environment:{name}")),
                    endpoint: Some(endpoint),
                };
            }
        }
    }

    #[cfg(target_os = "macos")]
    if let Some(endpoint) = detect_macos_proxy() {
        return ProxyDetection {
            detected: true,
            source: Some("macos-system".to_owned()),
            endpoint: Some(endpoint),
        };
    }

    ProxyDetection {
        detected: false,
        source: None,
        endpoint: None,
    }
}

fn validate_network_policy(policy: &NetworkPolicy) -> anyhow::Result<()> {
    if policy.max_retries > 8 {
        bail!("maximum retries cannot exceed 8");
    }
    if policy.retry_delay_ms == 0 || policy.max_retry_delay_ms < policy.retry_delay_ms {
        bail!("retry delay bounds are invalid");
    }
    if let ProxyMode::Manual { url } = &policy.proxy {
        let parsed = reqwest::Url::parse(url)?;
        if !matches!(parsed.scheme(), "http" | "https") {
            bail!("manual proxy URL must use http or https");
        }
        if !parsed.username().is_empty() || parsed.password().is_some() {
            bail!("manual proxy credentials must use the secret store");
        }
    }
    Ok(())
}

fn redact_proxy_url(value: &str) -> Option<String> {
    let mut url = reqwest::Url::parse(value).ok()?;
    if !matches!(url.scheme(), "http" | "https" | "socks5" | "socks5h") {
        return None;
    }
    url.set_username("").ok()?;
    url.set_password(None).ok()?;
    Some(url.to_string())
}

#[cfg(target_os = "macos")]
fn detect_macos_proxy() -> Option<String> {
    let output = std::process::Command::new("scutil")
        .args(["--proxy"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?;
    let mut enabled = false;
    let mut host = None;
    let mut port = None;
    for line in text.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        match key.trim() {
            "HTTPEnable" | "HTTPSEnable" if value.trim() == "1" => enabled = true,
            "HTTPProxy" | "HTTPSProxy" if !value.trim().is_empty() => host = Some(value.trim()),
            "HTTPPort" | "HTTPSPort" => port = value.trim().parse::<u16>().ok(),
            _ => {}
        }
    }
    if enabled {
        Some(format!("http://{}:{}", host?, port?))
    } else {
        None
    }
}

fn expand_home_path(path: String) -> anyhow::Result<PathBuf> {
    let path = path.trim();
    if path.is_empty() {
        bail!("download path cannot be empty");
    }

    if path == "~" || path.starts_with("~/") {
        let home =
            dirs::home_dir().ok_or_else(|| anyhow::anyhow!("home directory is unavailable"))?;
        return Ok(if path == "~" {
            home
        } else {
            home.join(&path[2..])
        });
    }

    Ok(PathBuf::from(path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_input_expands_home_and_uses_app_defaults() {
        let config = AppConfig::from_user_input("~/Downloads/dreamland".to_string()).unwrap();

        assert!(!config.download_path.to_string_lossy().starts_with("~"));
        assert_eq!(config.network, NetworkPolicy::default());
        assert_eq!(config.download_variant, MediaVariant::Full);
    }

    #[test]
    fn user_input_rejects_empty_paths() {
        assert!(AppConfig::from_user_input(" ".to_string()).is_err());
    }

    #[test]
    fn config_serializes_common_and_site_sections_as_toml() {
        let config = AppConfig::from_user_input("~/Downloads/dreamland".to_string()).unwrap();
        let encoded = toml::to_string_pretty(&config).unwrap();
        let decoded: AppConfig = toml::from_str(&encoded).unwrap();

        assert!(encoded.contains("[network]"));
        assert!(encoded.contains("[sites.yandere]"));
        assert!(decoded.sites["yandere"].enabled);
        assert!(!decoded.sites["pixiv"].enabled);
    }

    #[test]
    fn network_policy_rejects_proxy_credentials_and_excessive_retries() {
        let mut config = AppConfig::from_user_input("/tmp".to_string()).unwrap();
        config.network.proxy = ProxyMode::Manual {
            url: "http://user:password@proxy.example:8080".to_owned(),
        };
        assert!(config.validate().is_err());
        config.network.proxy = ProxyMode::Auto;
        config.network.max_retries = 9;
        assert!(config.validate().is_err());
    }

    #[test]
    fn proxy_detection_redacts_credentials() {
        assert_eq!(
            redact_proxy_url("http://user:password@proxy.example:8080").as_deref(),
            Some("http://proxy.example:8080/")
        );
    }
}
