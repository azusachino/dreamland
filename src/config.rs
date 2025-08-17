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
            .unwrap_or_else(|| dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")))
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
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("dreamland");
        
        let config_path = config_dir.join("config.json");
        
        if config_path.exists() {
            let content = std::fs::read_to_string(config_path)?;
            let config: AppConfig = serde_json::from_str(&content)?;
            Ok(config)
        } else {
            Ok(Self::default())
        }
    }
    
    pub fn save(&self) -> anyhow::Result<()> {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("dreamland");
        
        std::fs::create_dir_all(&config_dir)?;
        let config_path = config_dir.join("config.json");
        
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(config_path, content)?;
        Ok(())
    }
}