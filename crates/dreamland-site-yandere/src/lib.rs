use anyhow::Result;
use dreamland_core::{
    MediaVariant, Post, PostRef, Rating, SiteCapabilities, SiteDescriptor, SiteId,
};
use serde::{Deserialize, Serialize};

pub const SITE_ID: &str = "yandere";

const DEFAULT_CONFIG_TOML: &str = include_str!("../config/default.toml");

/// The adapter's default runtime configuration, owned as data (see
/// docs/adr/0005-toml-runtime-configuration.md) rather than a hardcoded Rust
/// literal, so the design isn't baked into source before API v1 settles.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct SiteDefaults {
    pub api_url: String,
}

pub fn default_config() -> SiteDefaults {
    toml::from_str(DEFAULT_CONFIG_TOML)
        .expect("bundled crates/dreamland-site-yandere/config/default.toml must parse")
}

pub fn descriptor() -> SiteDescriptor {
    SiteDescriptor {
        id: SiteId::new(SITE_ID),
        name: "Yande.re".to_owned(),
        capabilities: SiteCapabilities {
            browse: true,
            // Only paginated browse is implemented today. The rest stay
            // false until an adapter function actually backs them --
            // advertising them earlier already misled the API v1 draft.
            post_search: false,
            tag_search: false,
            tag_query: false,
            post_lookup: false,
            page_numbers: true,
            cursors: false,
            multiple_download_variants: true,
        },
    }
}

/// Resolve the download URL for a variant of an already-fetched post. Kept
/// alongside the wire type so the mapping from "variant name" to "site URL
/// field" lives in one place, owned by the adapter that knows the shape.
pub fn variant_url(post: &Post, variant: MediaVariant) -> &str {
    match variant {
        MediaVariant::Preview => &post.preview_url,
        MediaVariant::Sample => &post.sample_url,
        MediaVariant::Full => &post.full_url,
    }
}

/// Yande's raw `/post.json` wire shape. Never crosses the Tauri boundary --
/// `fetch_images` maps every entry into the site-neutral `Post` before
/// returning.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct ImagePost {
    id: u64,
    tags: String,
    width: u32,
    height: u32,
    file_url: String,
    sample_url: String,
    preview_url: String,
    rating: String,
    score: Option<i32>,
    file_size: Option<u64>,
}

impl From<ImagePost> for Post {
    fn from(image: ImagePost) -> Self {
        Self {
            post: PostRef {
                site: SiteId::new(SITE_ID),
                id: image.id.to_string(),
            },
            tags: image.tags.split_whitespace().map(str::to_owned).collect(),
            width: image.width,
            height: image.height,
            rating: match image.rating.as_str() {
                "s" => Rating::Safe,
                "q" => Rating::Questionable,
                "e" => Rating::Explicit,
                _ => Rating::Unknown,
            },
            score: image.score,
            preview_url: image.preview_url,
            sample_url: image.sample_url,
            full_url: image.file_url,
            file_size: image.file_size,
        }
    }
}

pub async fn fetch_images(url: &str, page: usize) -> Result<Vec<Post>> {
    let response = reqwest::Client::new()
        .get(url)
        .query(&[("page", page)])
        .send()
        .await?
        .error_for_status()?;

    let images: Vec<ImagePost> = response.json().await?;
    Ok(images.into_iter().map(Post::from).collect())
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
    fn site_uses_the_stable_yandere_id() {
        assert_eq!(SITE_ID, "yandere");
    }

    #[test]
    fn maps_wire_shape_to_neutral_post() {
        let image = ImagePost {
            id: 123456,
            tags: "tag1 tag2".to_owned(),
            width: 1920,
            height: 1080,
            file_url: "https://example.com/image.jpg".to_owned(),
            sample_url: "https://example.com/sample.jpg".to_owned(),
            preview_url: "https://example.com/preview.jpg".to_owned(),
            rating: "q".to_owned(),
            score: Some(42),
            file_size: Some(1_048_576),
        };

        let post = Post::from(image);

        assert_eq!(post.post.site.as_str(), SITE_ID);
        assert_eq!(post.post.id, "123456");
        assert_eq!(post.tags, vec!["tag1".to_owned(), "tag2".to_owned()]);
        assert_eq!(post.rating, Rating::Questionable);
        assert_eq!(variant_url(&post, MediaVariant::Full), post.full_url);
    }

    #[test]
    fn default_config_is_loaded_from_the_bundled_toml_not_hardcoded() {
        assert_eq!(default_config().api_url, "https://yande.re/post.json");
    }
}
