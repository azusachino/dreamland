use serde::{Deserialize, Serialize};
use anyhow::Result;

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
    let client = reqwest::Client::new();
    let response = client
        .get(url)
        .query(&[("page", page)])
        .send()
        .await?;
    
    let images: Vec<ImagePost> = response.json().await?;
    Ok(images)
}

pub async fn download_image(url: &str, download_dir: &std::path::Path) -> Result<(std::path::PathBuf, String)> {
    let client = reqwest::Client::new();
    let response = client.get(url).send().await?;
    
    // Get content type from response headers
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|ct| ct.to_str().ok())
        .unwrap_or("image/jpeg")
        .to_string();
    
    let bytes = response.bytes().await?;
    
    // Create a temporary filename
    let temp_filename = format!("temp_{}", uuid::Uuid::new_v4());
    let full_path = download_dir.join(temp_filename);
    
    tokio::fs::write(&full_path, bytes).await?;
    
    Ok((full_path, content_type))
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
    fn test_image_post_deserialize() {
        let json_data = r#"
        {
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
        }"#;

        let image_post: Result<ImagePost, _> = serde_json::from_str(json_data);
        assert!(image_post.is_ok());
        
        let image = image_post.unwrap();
        assert_eq!(image.id, 123456);
        assert_eq!(image.tags, "test tag1 tag2");
        assert_eq!(image.width, 1920);
        assert_eq!(image.height, 1080);
        assert_eq!(image.rating, "s");
    }

    #[test]
    fn test_image_post_array_deserialize() {
        let json_data = r#"
        [
            {
                "id": 1,
                "tags": "tag1",
                "width": 800,
                "height": 600,
                "file_url": "https://example.com/1.jpg",
                "sample_url": "https://example.com/s1.jpg",
                "preview_url": "https://example.com/p1.jpg",
                "rating": "s",
                "score": 10,
                "md5": "abc123",
                "file_size": 50000
            },
            {
                "id": 2,
                "tags": "tag2",
                "width": 1024,
                "height": 768,
                "file_url": "https://example.com/2.png",
                "sample_url": "https://example.com/s2.png",
                "preview_url": "https://example.com/p2.png",
                "rating": "q",
                "score": 20,
                "md5": "def456",
                "file_size": 75000
            }
        ]"#;

        let images: Result<Vec<ImagePost>, _> = serde_json::from_str(json_data);
        assert!(images.is_ok());
        
        let image_list = images.unwrap();
        assert_eq!(image_list.len(), 2);
        assert_eq!(image_list[0].id, 1);
        assert_eq!(image_list[1].id, 2);
    }
}