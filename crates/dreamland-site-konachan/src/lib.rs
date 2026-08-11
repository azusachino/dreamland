use anyhow::{bail, Result};
use dreamland_core::{
    ContentPolicy, MediaVariant, NetworkPolicy, Post, PostQueryRequest, SiteCapabilities,
    SiteDescriptor, SiteId, SitePage, TagSuggestion, TagSuggestionRequest,
};
use serde::Deserialize;

pub const SITE_ID: &str = "konachan";

const DEFAULT_CONFIG_TOML: &str = include_str!("../config/default.toml");

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct SiteDefaults {
    pub api_url: String,
    pub browser_url: String,
    pub safe_only: bool,
}

pub fn default_config() -> SiteDefaults {
    toml::from_str(DEFAULT_CONFIG_TOML)
        .expect("bundled crates/dreamland-site-konachan/config/default.toml must parse")
}

pub fn descriptor() -> SiteDescriptor {
    SiteDescriptor {
        id: SiteId::new(SITE_ID),
        name: "konachan".to_owned(),
        capabilities: SiteCapabilities {
            browse: true,
            post_search: true,
            tag_search: true,
            tag_query: true,
            post_lookup: false,
            page_numbers: true,
            cursors: false,
            multiple_download_variants: true,
            safe_content_only: true,
            authentication: false,
            remote_favorites: false,
            favorite_list: false,
            collections: false,
            collection_downloads: false,
        },
    }
}

pub fn variant_url(post: &Post, variant: MediaVariant) -> Option<&str> {
    match variant {
        MediaVariant::Preview => post.preview_url.as_deref(),
        MediaVariant::Sample => post.sample_url.as_deref(),
        MediaVariant::Full => post.full_url.as_deref(),
    }
}

pub fn decode_posts(body: &[u8]) -> Result<Vec<Post>> {
    let mut posts = dreamland_site_yandere::decode_posts(body)?;
    for post in &mut posts {
        post.post.site = SiteId::new(SITE_ID);
    }
    Ok(posts)
}

pub async fn query_posts(
    api_url: &str,
    request: &PostQueryRequest,
    network: &NetworkPolicy,
) -> Result<SitePage> {
    ensure_supported_policy(request.query.content_policy)?;
    dreamland_site_yandere::query_posts(api_url, request, network)
        .await
        .map_err(map_transport_error)
        .map(|mut page| {
            for post in &mut page.posts {
                post.post.site = SiteId::new(SITE_ID);
            }
            page
        })
}

pub async fn fetch_tag_suggestions(
    endpoint: &str,
    request: &TagSuggestionRequest,
    network: &NetworkPolicy,
) -> Result<Vec<TagSuggestion>> {
    dreamland_site_yandere::fetch_tag_suggestions(endpoint, request, network)
        .await
        .map_err(map_transport_error)
}

pub fn tag_endpoint(base_url: &str) -> Result<String> {
    dreamland_site_yandere::tag_endpoint(base_url)
}

fn ensure_supported_policy(policy: ContentPolicy) -> Result<()> {
    if default_config().safe_only && policy != ContentPolicy::SafeOnly {
        bail!("konachan safe API supports Safe only; open the konachan browser page for explicit-host access")
    }
    Ok(())
}

fn map_transport_error(error: anyhow::Error) -> anyhow::Error {
    let message = error.to_string();
    if message.contains("HTTP 403") || message.contains("HTTP 503") {
        anyhow::anyhow!(
            "konachan API request was blocked by bot protection; open {} in a browser and retry later",
            default_config().browser_url
        )
    } else {
        anyhow::anyhow!("konachan API request failed: {message}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const KONACHAN_POST: &str = r#"{
        "id": 407162,
        "tags": "barefoot blue_eyes cirno dress touhou",
        "created_at": 1786390664,
        "creator_id": 73632,
        "author": "otaku_emmy",
        "source": "https://ryosios.fanbox.cc/posts/3418182",
        "score": 15,
        "md5": "512100c4db5c913ec30c6ed3c5b913d6",
        "file_size": 27856893,
        "file_url": "https://konachan.net/image/512100c4db5c913ec30c6ed3c5b913d6/file.png",
        "sample_url": "https://konachan.net/sample/512100c4db5c913ec30c6ed3c5b913d6/file.jpg",
        "preview_url": "https://konachan.net/data/preview/51/21/file.jpg",
        "rating": "s",
        "width": 5302,
        "height": 2832,
        "parent_id": null,
        "has_children": false
    }"#;

    #[test]
    fn descriptor_is_browse_capable_but_safe_only_is_explicit() {
        assert_eq!(SITE_ID, "konachan");
        assert!(descriptor().capabilities.browse);
        assert!(default_config().safe_only);
        assert!(ensure_supported_policy(ContentPolicy::AllowExplicit).is_err());
    }

    #[test]
    fn decodes_real_konachan_numeric_timestamp_shape() {
        let posts = decode_posts(format!("[{KONACHAN_POST}]").as_bytes()).unwrap();

        assert_eq!(posts[0].post.site.as_str(), SITE_ID);
        assert_eq!(posts[0].post.id, "407162");
        assert_eq!(posts[0].author.as_deref(), Some("otaku_emmy"));
        assert_eq!(posts[0].created_at.as_deref(), Some("2026-08-10T19:37:44Z"));
        assert_eq!(posts[0].rating, dreamland_core::Rating::Safe);
    }

    #[test]
    fn config_keeps_browser_and_safe_api_origins_separate() {
        let config = default_config();

        assert_eq!(config.api_url, "https://konachan.net/post.json");
        assert_eq!(config.browser_url, "https://konachan.com/post");
    }
}
