use anyhow::Result;
use serde::{Deserialize, Serialize};

pub const PROVIDER_ID: &str = "yandere";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
    let response = reqwest::Client::new()
        .get(url)
        .query(&[("page", page)])
        .send()
        .await?
        .error_for_status()?;

    Ok(response.json().await?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_post_deserializes() {
        let image: ImagePost = serde_json::from_str(
            r#"{
                "id": 123456,
                "tags": "test tag1 tag2",
                "width": 1920,
                "height": 1080,
                "file_url": "https://example.com/image.jpg",
                "sample_url": "https://example.com/sample.jpg",
                "preview_url": "https://example.com/preview.jpg",
                "rating": "s",
                "score": 42,
                "md5": "abcdef123456",
                "file_size": 1048576
            }"#,
        )
        .unwrap();

        assert_eq!(image.id, 123456);
        assert_eq!(image.rating, "s");
    }

    #[test]
    fn provider_uses_the_stable_yandere_id() {
        assert_eq!(PROVIDER_ID, "yandere");
    }
}
