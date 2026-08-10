use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub download_path: PathBuf,
    pub api_url: String,
    pub images_per_page: usize,
}

impl Default for AppConfig {
    fn default() -> Self {
        let download_path = dirs::download_dir()
            .or_else(dirs::home_dir)
            .unwrap_or_else(|| PathBuf::from("."))
            .join("dreamland_images");

        Self {
            download_path,
            api_url: "https://yande.re/post.json".to_string(),
            images_per_page: 20,
        }
    }
}

impl AppConfig {
    pub fn load() -> anyhow::Result<Self> {
        let config_path = Self::config_dir().join("config.json");
        if config_path.exists() {
            Ok(serde_json::from_str(&std::fs::read_to_string(
                config_path,
            )?)?)
        } else {
            Ok(Self::default())
        }
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let config_dir = Self::config_dir();
        std::fs::create_dir_all(&config_dir)?;
        std::fs::write(
            config_dir.join("config.json"),
            serde_json::to_string_pretty(self)?,
        )?;
        Ok(())
    }

    fn config_dir() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("dreamland")
    }
}
