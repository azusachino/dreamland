use anyhow::bail;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub download_path: PathBuf,
    pub api_url: String,
    #[serde(default = "default_images_per_page")]
    pub images_per_page: usize,
}

fn default_images_per_page() -> usize {
    20
}

fn default_download_path() -> PathBuf {
    dirs::download_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("dreamland_images")
}

impl AppConfig {
    pub fn from_user_input(download_path: String, api_url: String) -> anyhow::Result<Self> {
        let download_path = expand_home_path(download_path)?;
        let api_url = validate_api_url(api_url)?;

        Ok(Self {
            download_path,
            api_url,
            images_per_page: default_images_per_page(),
        })
    }

    /// Load the saved config, or build a default seeded with `default_api_url`.
    /// The runtime stays site-neutral: the caller (the composition point that
    /// already knows which site is active) supplies the default URL rather
    /// than this crate hard-coding one.
    pub fn load_or_default(default_api_url: &str) -> anyhow::Result<Self> {
        let config_path = Self::config_dir().join("config.json");
        if config_path.exists() {
            let mut config: Self = serde_json::from_str(&std::fs::read_to_string(config_path)?)?;
            config.download_path =
                expand_home_path(config.download_path.to_string_lossy().into_owned())?;
            config.validate()?;
            Ok(config)
        } else {
            Ok(Self {
                download_path: default_download_path(),
                api_url: validate_api_url(default_api_url.to_string())?,
                images_per_page: default_images_per_page(),
            })
        }
    }

    pub fn save(&self) -> anyhow::Result<()> {
        self.validate()?;
        let config_dir = Self::config_dir();
        std::fs::create_dir_all(&config_dir)?;
        std::fs::write(
            config_dir.join("config.json"),
            serde_json::to_string_pretty(self)?,
        )?;
        Ok(())
    }

    fn validate(&self) -> anyhow::Result<()> {
        if self.download_path.as_os_str().is_empty() {
            bail!("download path cannot be empty");
        }
        validate_api_url(self.api_url.clone()).map(|_| ())
    }

    fn config_dir() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("dreamland")
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

fn validate_api_url(value: String) -> anyhow::Result<String> {
    let value = value.trim();
    let url = reqwest::Url::parse(value)?;
    if !matches!(url.scheme(), "http" | "https") {
        bail!("API URL must use http or https");
    }
    Ok(url.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_input_expands_home_and_validates_url() {
        let config = AppConfig::from_user_input(
            "~/Downloads/dreamland".to_string(),
            "https://yande.re/post.json".to_string(),
        )
        .unwrap();

        assert!(!config.download_path.to_string_lossy().starts_with("~"));
        assert_eq!(config.api_url, "https://yande.re/post.json");
    }

    #[test]
    fn user_input_rejects_empty_paths_and_non_http_urls() {
        assert!(
            AppConfig::from_user_input(" ".to_string(), "https://example.com".to_string()).is_err()
        );
        assert!(AppConfig::from_user_input("/tmp".to_string(), "file:///tmp".to_string()).is_err());
    }
}
