use serde::{Deserialize, Serialize};
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImagePost {
    pub id: u64,
    pub tags: String,
    pub width: u32,
    pub height: u32,
    pub file_url: String,
    pub sample_url: String,
    pub preview_url: String,
    pub rating: String,
    pub score: Option<i32>,
    pub md5: String,
    pub file_size: Option<u64>,
}

pub async fn fetch_images(url: &str, page: usize) -> Result<Vec<ImagePost>> {
    let client = reqwest::Client::new();
    let response = client
        .get(url)
        .query(&[("page", page)])
        .send()
        .await?;
    
    let images: Vec<ImagePost> = response.json().await?;
    Ok(images)
}

pub async fn download_image(url: &str, path: &str) -> Result<()> {
    let client = reqwest::Client::new();
    let response = client.get(url).send().await?;
    let bytes = response.bytes().await?;
    
    tokio::fs::write(path, bytes).await?;
    Ok(())
}