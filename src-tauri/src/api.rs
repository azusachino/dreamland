use anyhow::Result;
use serde::{Deserialize, Serialize};

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
        .await?;

    Ok(response.json().await?)
}

pub async fn download_file(
    url: &str,
    download_dir: &std::path::Path,
) -> Result<(std::path::PathBuf, String)> {
    let response = reqwest::Client::new().get(url).send().await?;
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("image/jpeg")
        .split(';')
        .next()
        .unwrap_or("image/jpeg")
        .to_string();
    let temporary_path = download_dir.join(format!("temp_{}", uuid::Uuid::new_v4()));

    tokio::fs::write(&temporary_path, response.bytes().await?).await?;
    Ok((temporary_path, content_type))
}

pub fn mime_to_extension(content_type: &str) -> Option<&str> {
    match content_type {
        "image/jpeg" => Some("jpg"),
        "image/png" => Some("png"),
        "image/gif" => Some("gif"),
        "image/webp" => Some("webp"),
        "image/bmp" => Some("bmp"),
        "image/svg+xml" => Some("svg"),
        _ => None,
    }
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
    fn mime_types_map_to_extensions() {
        assert_eq!(mime_to_extension("image/jpeg"), Some("jpg"));
        assert_eq!(mime_to_extension("image/unknown"), None);
    }
}
