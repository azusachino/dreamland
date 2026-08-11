use anyhow::{bail, Context, Result};
use dreamland_core::{
    ContentPolicy, MediaVariant, NetworkPolicy, Pool, PoolPage, Post, PostQueryRequest,
    SiteCapabilities, SiteDescriptor, SiteId, SitePage, TagSuggestion, TagSuggestionRequest,
};
use serde::Deserialize;

pub const SITE_ID: &str = "konachan";
const MAX_POOL_PAGE_SIZE: u16 = 20;

const DEFAULT_CONFIG_TOML: &str = include_str!("../config/default.toml");

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct SiteDefaults {
    pub api_url: String,
    pub browser_url: String,
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
            related_tags: true,
            tag_query: true,
            post_lookup: true,
            similar_search: true,
            page_numbers: true,
            cursors: false,
            multiple_download_variants: true,
            safe_content_only: false,
            authentication: false,
            remote_favorites: false,
            favorite_list: false,
            collections: true,
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
    dreamland_moe::decode_posts(body, SITE_ID)
}

pub async fn query_posts(
    api_url: &str,
    request: &PostQueryRequest,
    network: &NetworkPolicy,
) -> Result<SitePage> {
    dreamland_moe::query_posts(api_url, request, network, SITE_ID, "konachan")
        .await
        .map_err(map_transport_error)
        .map(|mut page| {
            for post in &mut page.posts {
                post.post.site = SiteId::new(SITE_ID);
            }
            page
        })
}

pub async fn lookup_post(
    api_url: &str,
    post_id: &str,
    content_policy: ContentPolicy,
    network: &NetworkPolicy,
) -> Result<Post> {
    let request = lookup_post_request(post_id, content_policy)?;
    let page = query_posts(api_url, &request, network).await?;
    page.posts
        .into_iter()
        .find(|post| post.post.id == post_id)
        .ok_or_else(|| anyhow::anyhow!("Konachan post #{post_id} was not found in the API"))
}

fn lookup_post_request(post_id: &str, content_policy: ContentPolicy) -> Result<PostQueryRequest> {
    if post_id.is_empty() || !post_id.chars().all(|value| value.is_ascii_digit()) {
        bail!("Konachan post id must be numeric");
    }
    Ok(PostQueryRequest {
        query: dreamland_core::ReplayableQuery {
            source: dreamland_core::DiscoverySource::Search {
                expression: format!("id:{post_id}"),
            },
            content_policy,
        },
        pagination: dreamland_core::PaginationRequest::First { page_size: 1 },
    })
}

pub async fn fetch_tag_suggestions(
    endpoint: &str,
    request: &TagSuggestionRequest,
    network: &NetworkPolicy,
) -> Result<Vec<TagSuggestion>> {
    dreamland_moe::fetch_tag_suggestions(endpoint, request, network, "konachan")
        .await
        .map_err(map_transport_error)
}

pub fn tag_endpoint(base_url: &str) -> Result<String> {
    dreamland_moe::tag_endpoint(base_url)
}

pub fn related_tag_endpoint(base_url: &str) -> Result<String> {
    dreamland_moe::related_tag_endpoint(base_url)
}

pub async fn fetch_related_tags(
    endpoint: &str,
    request: &dreamland_core::RelatedTagRequest,
    network: &NetworkPolicy,
) -> Result<Vec<dreamland_core::RelatedTag>> {
    dreamland_moe::fetch_related_tags(endpoint, request, network, SITE_ID)
        .await
        .map_err(map_transport_error)
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
struct PoolRecord {
    id: u64,
    name: String,
    #[serde(default)]
    is_public: bool,
    #[serde(default)]
    post_count: u32,
}

pub fn pool_endpoint(base_url: &str) -> Result<String> {
    let mut url = reqwest::Url::parse(base_url).context("parse Konachan API URL")?;
    url.set_path("/pool.json");
    url.set_query(None);
    Ok(url.to_string())
}

pub fn browser_post_url(browser_url: &str, post_id: &str) -> Result<String> {
    validate_post_id(post_id)?;
    let mut url = reqwest::Url::parse(browser_url).context("parse Konachan browser URL")?;
    url.set_path(&format!("/post/show/{post_id}"));
    url.set_query(None);
    Ok(url.to_string())
}

pub fn browser_similar_url(browser_url: &str) -> Result<String> {
    let mut url = reqwest::Url::parse(browser_url).context("parse Konachan browser URL")?;
    url.set_path("/post/similar");
    url.set_query(None);
    Ok(url.to_string())
}

pub fn pool_posts_endpoint(base_url: &str) -> Result<String> {
    let mut url = reqwest::Url::parse(base_url).context("parse Konachan API URL")?;
    url.set_path("/pool/show.json");
    url.set_query(None);
    Ok(url.to_string())
}

fn validate_pool_id(pool_id: &str) -> Result<()> {
    if pool_id.is_empty() || !pool_id.chars().all(|value| value.is_ascii_digit()) {
        bail!("Konachan pool id must be numeric");
    }
    Ok(())
}

fn validate_post_id(post_id: &str) -> Result<()> {
    if post_id.is_empty() || !post_id.chars().all(|value| value.is_ascii_digit()) {
        bail!("Konachan post id must be numeric");
    }
    Ok(())
}

pub fn decode_pools(body: &[u8]) -> Result<Vec<Pool>> {
    let records =
        serde_json::from_slice::<Vec<PoolRecord>>(body).context("decode Konachan pools")?;
    Ok(records
        .into_iter()
        .map(|pool| Pool {
            site: SiteId::new(SITE_ID),
            id: pool.id.to_string(),
            name: pool.name,
            post_count: pool.post_count,
            public: pool.is_public,
        })
        .collect())
}

pub async fn fetch_pools(
    api_url: &str,
    query: &str,
    page: u32,
    page_size: u16,
    network: &NetworkPolicy,
) -> Result<PoolPage> {
    if page == 0 || page_size == 0 {
        bail!("Konachan pool pagination must be greater than zero");
    }
    let endpoint = pool_endpoint(api_url)?;
    let page_size = page_size.min(MAX_POOL_PAGE_SIZE);
    let params = dreamland_moe::pool_query_params(query, page, page_size);
    let pools = decode_pools(
        &dreamland_moe::fetch_bytes(&endpoint, &params, network, "konachan")
            .await
            .map_err(map_transport_error)?,
    )?;
    Ok(PoolPage {
        has_next: pools.len() == usize::from(page_size),
        pools,
        page,
        page_size,
    })
}

pub fn decode_pool_posts(body: &[u8]) -> Result<Vec<Post>> {
    dreamland_moe::decode_posts(body, SITE_ID)
}

pub async fn query_pool_posts(
    api_url: &str,
    pool_id: &str,
    content_policy: ContentPolicy,
    page: u32,
    page_size: u16,
    network: &NetworkPolicy,
) -> Result<SitePage> {
    validate_pool_id(pool_id)?;
    if page == 0 || page_size == 0 {
        bail!("Konachan pool pagination must be greater than zero");
    }
    let endpoint = pool_posts_endpoint(api_url)?;
    let params = [("id", pool_id.to_owned()), ("page", page.to_string())];
    let mut posts = decode_pool_posts(
        &dreamland_moe::fetch_bytes(&endpoint, &params, network, "konachan")
            .await
            .map_err(map_transport_error)?,
    )?;
    dreamland_moe::retain_content_policy(&mut posts, content_policy);
    let total = u64::try_from(posts.len()).unwrap_or(u64::MAX);
    Ok(SitePage {
        posts,
        continuation: dreamland_core::Continuation::None,
        total: Some(total),
        page_size,
        session: None,
    })
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
    fn descriptor_exposes_real_konachan_content_policy() {
        assert_eq!(SITE_ID, "konachan");
        assert!(descriptor().capabilities.browse);
        assert!(descriptor().capabilities.collections);
        assert!(descriptor().capabilities.post_lookup);
        assert!(!descriptor().capabilities.collection_downloads);
        assert!(!descriptor().capabilities.safe_content_only);
    }

    #[test]
    fn lookup_requires_numeric_id() {
        assert!(lookup_post_request("not-a-number", ContentPolicy::AllowExplicit).is_err());
        assert_eq!(
            lookup_post_request("407162", ContentPolicy::AllowExplicit)
                .unwrap()
                .query
                .source,
            dreamland_core::DiscoverySource::Search {
                expression: "id:407162".to_owned()
            }
        );
        assert_eq!(
            lookup_post_request("407162", ContentPolicy::AllowExplicit)
                .unwrap()
                .query
                .content_policy,
            ContentPolicy::AllowExplicit
        );
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
    fn config_uses_real_api_and_browser_origins() {
        let config = default_config();

        assert_eq!(config.api_url, "https://konachan.com/post.json");
        assert_eq!(config.browser_url, "https://konachan.com/post");
        assert_eq!(
            browser_post_url(&config.browser_url, "407162").unwrap(),
            "https://konachan.com/post/show/407162"
        );
        assert_eq!(
            browser_similar_url(&config.browser_url).unwrap(),
            "https://konachan.com/post/similar"
        );
        assert!(browser_post_url(&config.browser_url, "not-a-number").is_err());
    }

    #[test]
    fn pool_endpoints_are_owned_by_the_real_api_origin() {
        let config = default_config();

        assert_eq!(
            pool_endpoint(&config.api_url).unwrap(),
            "https://konachan.com/pool.json"
        );
        assert_eq!(
            pool_posts_endpoint(&config.api_url).unwrap(),
            "https://konachan.com/pool/show.json"
        );
        assert_eq!(
            related_tag_endpoint(&config.api_url).unwrap(),
            "https://konachan.com/tag/related.json"
        );
        assert!(validate_pool_id("not-a-number").is_err());
    }

    #[test]
    fn decodes_public_pool_metadata_and_ordered_posts() {
        let pools = decode_pools(
            br#"[{"id":556,"name":"Reverse_Yogic_Sleep_Pose","is_public":true,"post_count":10}]"#,
        )
        .unwrap();
        assert_eq!(pools[0].site.as_str(), SITE_ID);
        assert_eq!(pools[0].id, "556");
        assert!(pools[0].public);
        assert_eq!(pools[0].post_count, 10);

        let posts = decode_pool_posts(
            format!(r#"{{"id":556,"post_count":1,"posts":[{KONACHAN_POST}]}}"#).as_bytes(),
        )
        .unwrap();
        assert_eq!(posts.len(), 1);
        assert_eq!(posts[0].post.site.as_str(), SITE_ID);
        assert_eq!(posts[0].post.id, "407162");
    }
}
