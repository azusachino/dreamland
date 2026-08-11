use anyhow::{bail, Context, Result};
use dreamland_core::{
    ContentPolicy, DiscoverySource, MediaVariant, NetworkPolicy, PaginationRequest, Pool, PoolPage,
    PopularPeriod, Post, PostQueryRequest, ProxyMode, SiteCapabilities, SiteDescriptor, SiteError,
    SiteId, SitePage, TagSuggestion, TagSuggestionRequest,
};
use serde::Deserialize;

pub const SITE_ID: &str = "yandere";
const MAX_POOL_PAGE_SIZE: u16 = 20;
const USER_AGENT: &str = concat!("Dreamland/", env!("CARGO_PKG_VERSION"));

const DEFAULT_CONFIG_TOML: &str = include_str!("../config/default.toml");

/// The adapter's default runtime configuration, owned as data (see
/// docs/adr/0005-toml-runtime-configuration.md) rather than a hardcoded Rust
/// literal, so the design isn't baked into source before API v1 settles.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct SiteDefaults {
    pub api_url: String,
    pub browser_url: String,
}

pub fn default_config() -> SiteDefaults {
    toml::from_str(DEFAULT_CONFIG_TOML)
        .expect("bundled crates/dreamland-site-yandere/config/default.toml must parse")
}

pub fn descriptor() -> SiteDescriptor {
    SiteDescriptor {
        id: SiteId::new(SITE_ID),
        name: "yandere".to_owned(),
        capabilities: SiteCapabilities {
            browse: true,
            post_search: true,
            tag_search: true,
            related_tags: true,
            tag_query: true,
            post_lookup: false,
            similar_search: false,
            page_numbers: true,
            cursors: false,
            multiple_download_variants: true,
            safe_content_only: false,
            authentication: true,
            remote_favorites: true,
            favorite_list: true,
            collections: true,
            collection_downloads: true,
        },
    }
}

/// Resolve the download URL for a variant of an already-fetched post. Kept
/// alongside the wire type so the mapping from "variant name" to "site URL
/// field" lives in one place, owned by the adapter that knows the shape.
pub fn variant_url(post: &Post, variant: MediaVariant) -> Option<&str> {
    match variant {
        MediaVariant::Preview => post.preview_url.as_deref(),
        MediaVariant::Sample => post.sample_url.as_deref(),
        MediaVariant::Full => post.full_url.as_deref(),
    }
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

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
struct UserRecord {
    name: String,
    id: u64,
}

/// Decode both Yande post response envelopes observed in the live API.
pub fn decode_posts(body: &[u8]) -> Result<Vec<Post>> {
    dreamland_moe::decode_posts(body, SITE_ID)
}

/// Keep the site's tag expression intact while adding the runtime's content
/// policy. In particular, spaces and leading `-` terms are meaningful Yande
/// syntax and must not be normalized into a different query.
pub fn tag_expression(expression: &str, policy: ContentPolicy) -> Result<String> {
    dreamland_moe::tag_expression(expression, policy)
}

pub fn tag_query_params(
    expression: &str,
    policy: ContentPolicy,
    page: u32,
    page_size: u16,
) -> Result<Vec<(&'static str, String)>> {
    dreamland_moe::tag_query_params(expression, policy, page, page_size)
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
    dreamland_moe::fetch_related_tags(endpoint, request, network, SITE_ID).await
}

pub fn pool_endpoint(base_url: &str) -> Result<String> {
    let mut url = reqwest::Url::parse(base_url).context("parse Yande API URL")?;
    url.set_path("/pool.json");
    url.set_query(None);
    Ok(url.to_string())
}

pub fn browser_post_url(browser_url: &str, post_id: &str) -> Result<String> {
    if post_id.is_empty() || !post_id.chars().all(|value| value.is_ascii_digit()) {
        bail!("Yande post id must be numeric");
    }
    let mut url = reqwest::Url::parse(browser_url).context("parse Yande browser URL")?;
    url.set_path(&format!("/post/show/{post_id}"));
    url.set_query(None);
    Ok(url.to_string())
}

pub fn pool_posts_expression(pool_id: &str) -> Result<String> {
    if pool_id.is_empty() || !pool_id.chars().all(|value| value.is_ascii_digit()) {
        bail!("Yande pool id must be numeric");
    }
    Ok(format!("pool:{pool_id}"))
}

pub fn pool_zip_endpoint(base_url: &str, pool_id: &str) -> Result<String> {
    if pool_id.is_empty() || !pool_id.chars().all(|value| value.is_ascii_digit()) {
        bail!("Yande pool id must be numeric");
    }
    let mut url = reqwest::Url::parse(base_url).context("parse Yande API URL")?;
    url.set_path(&format!("/pool/zip/{pool_id}"));
    url.set_query(None);
    Ok(url.to_string())
}

pub fn decode_pools(body: &[u8]) -> Result<Vec<Pool>> {
    let records = serde_json::from_slice::<Vec<PoolRecord>>(body).context("decode Yande pools")?;
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
        bail!("Yande pool pagination must be greater than zero");
    }
    let endpoint = pool_endpoint(api_url)?;
    let client = build_client(network)?;
    let page_size = page_size.min(MAX_POOL_PAGE_SIZE);
    let params = dreamland_moe::pool_query_params(query, page, page_size);
    let pools = decode_pools(&get_bytes(&client, &endpoint, &params, network).await?)?;
    Ok(PoolPage {
        has_next: pools.len() == usize::from(page_size),
        pools,
        page,
        page_size,
    })
}

pub async fn current_user(
    api_url: &str,
    user_id: &str,
    cookie_header: &str,
    network: &NetworkPolicy,
) -> Result<String> {
    if user_id.is_empty() || !user_id.chars().all(|value| value.is_ascii_digit()) {
        bail!("Yande user id must be numeric");
    }
    let mut endpoint = reqwest::Url::parse(api_url).context("parse Yande API URL")?;
    endpoint.set_path("/user.json");
    endpoint.set_query(None);
    let client = build_client_with_cookie(network, Some(cookie_header))?;
    let users = serde_json::from_slice::<Vec<UserRecord>>(
        &get_bytes(
            &client,
            endpoint.as_str(),
            &[("id", user_id.to_owned()), ("limit", "1".to_owned())],
            network,
        )
        .await?,
    )
    .context("decode Yande current user")?;
    users
        .into_iter()
        .find(|user| user.id.to_string() == user_id)
        .map(|user| user.name)
        .ok_or_else(|| anyhow::anyhow!("Yande current user was not found"))
}

pub async fn query_pool_posts(
    api_url: &str,
    pool_id: &str,
    content_policy: ContentPolicy,
    page: u32,
    page_size: u16,
    network: &NetworkPolicy,
) -> Result<SitePage> {
    let request = PostQueryRequest {
        query: dreamland_core::ReplayableQuery {
            source: DiscoverySource::Search {
                expression: pool_posts_expression(pool_id)?,
            },
            content_policy,
        },
        pagination: PaginationRequest::Page {
            number: page,
            page_size,
        },
    };
    query_posts(api_url, &request, network).await
}

pub fn popular_tag_expression(period: PopularPeriod, anchor_date: &str) -> Result<String> {
    dreamland_moe::popular_tag_expression(period, anchor_date)
}

pub fn map_http_status(status: reqwest::StatusCode) -> SiteError {
    dreamland_moe::map_http_status(status, "yandere")
}

pub async fn query_posts(
    api_url: &str,
    request: &PostQueryRequest,
    network: &NetworkPolicy,
) -> Result<SitePage> {
    dreamland_moe::query_posts(api_url, request, network, SITE_ID, "yandere").await
}

pub async fn list_favorites(
    api_url: &str,
    username: &str,
    cookie_header: &str,
    content_policy: ContentPolicy,
    page: u32,
    page_size: u16,
    network: &NetworkPolicy,
) -> Result<SitePage> {
    let request = PostQueryRequest {
        query: dreamland_core::ReplayableQuery {
            source: DiscoverySource::Search {
                expression: format!("vote:3:{username} order:vote"),
            },
            content_policy,
        },
        pagination: PaginationRequest::Page {
            number: page,
            page_size,
        },
    };
    query_posts_with_cookie(api_url, &request, network, Some(cookie_header)).await
}

async fn query_posts_with_cookie(
    api_url: &str,
    request: &PostQueryRequest,
    network: &NetworkPolicy,
    cookie_header: Option<&str>,
) -> Result<SitePage> {
    dreamland_moe::query_posts_with_cookie(
        api_url,
        request,
        network,
        cookie_header,
        SITE_ID,
        "yandere",
    )
    .await
}

fn build_client(network: &NetworkPolicy) -> Result<reqwest::Client> {
    build_client_with_cookie(network, None)
}

fn build_client_with_cookie(
    network: &NetworkPolicy,
    cookie_header: Option<&str>,
) -> Result<reqwest::Client> {
    let mut builder = reqwest::Client::builder().user_agent(USER_AGENT);
    match &network.proxy {
        ProxyMode::Auto => {}
        ProxyMode::Direct => builder = builder.no_proxy(),
        ProxyMode::Manual { url } => builder = builder.proxy(reqwest::Proxy::all(url)?),
    }
    if let Some(cookie_header) = cookie_header {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::COOKIE,
            reqwest::header::HeaderValue::from_str(cookie_header)?,
        );
        builder = builder.default_headers(headers);
    }
    Ok(builder.build()?)
}

pub async fn set_favorite(
    api_url: &str,
    post_id: &str,
    favorite: bool,
    cookie_header: &str,
    network: &NetworkPolicy,
) -> Result<()> {
    dreamland_moe::set_favorite(
        api_url,
        post_id,
        favorite,
        cookie_header,
        network,
        "yandere",
    )
    .await
}

async fn get_bytes(
    client: &reqwest::Client,
    endpoint: &str,
    params: &[(&str, String)],
    network: &NetworkPolicy,
) -> Result<Vec<u8>> {
    let mut retry = 0;
    loop {
        let response = client.get(endpoint).query(params).send().await?;
        if response.status().is_success() {
            return Ok(response.bytes().await?.to_vec());
        }

        let status = response.status();
        if retry >= network.max_retries || !matches!(status.as_u16(), 429 | 500 | 502 | 503 | 504) {
            let error = map_http_status(status);
            bail!("{}", error.message);
        }
        let delay_ms = retry_after_ms(&response)
            .unwrap_or_else(|| {
                network
                    .retry_delay_ms
                    .saturating_mul(1_u64 << retry.min(16))
            })
            .min(network.max_retry_delay_ms);
        tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
        retry += 1;
    }
}

fn retry_after_ms(response: &reqwest::Response) -> Option<u64> {
    response
        .headers()
        .get(reqwest::header::RETRY_AFTER)?
        .to_str()
        .ok()?
        .parse::<u64>()
        .ok()
        .map(|seconds| seconds.saturating_mul(1_000))
}

#[cfg(test)]
fn request_parts(
    api_url: &str,
    request: &PostQueryRequest,
) -> Result<(String, Vec<(&'static str, String)>, u16, bool)> {
    dreamland_moe::request_parts(api_url, request)
}

pub fn decode_tag_suggestions(body: &[u8]) -> Result<Vec<TagSuggestion>> {
    dreamland_moe::decode_tag_suggestions(body)
}

pub async fn fetch_tag_suggestions(
    url: &str,
    request: &TagSuggestionRequest,
    network: &NetworkPolicy,
) -> Result<Vec<TagSuggestion>> {
    dreamland_moe::fetch_tag_suggestions(url, request, network, "yandere").await
}

pub async fn fetch_images(url: &str, page: usize) -> Result<Vec<Post>> {
    dreamland_moe::fetch_images(url, page, SITE_ID).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use dreamland_core::{FeedKind, Rating, SiteErrorCode, TagCategory};

    const IMAGE_JSON: &str = r#"{
        "id": 123456,
        "tags": "test tag1 tag2",
        "author": "board_user",
        "creator_id": 452674,
        "source": "https://www.pixiv.net/artworks/148271180",
        "parent_id": 123455,
        "has_children": true,
        "created_at": "2026-08-11T01:02:03.000Z",
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

    #[test]
    fn site_uses_the_stable_yandere_id() {
        assert_eq!(SITE_ID, "yandere");
    }

    #[test]
    fn maps_wire_shape_to_neutral_post() {
        let posts = decode_posts(format!("[{IMAGE_JSON}]").as_bytes()).unwrap();
        let post = &posts[0];

        assert_eq!(post.post.site.as_str(), SITE_ID);
        assert_eq!(post.post.id, "123456");
        assert_eq!(post.tags, vec!["test", "tag1", "tag2"]);
        assert_eq!(post.author.as_deref(), Some("board_user"));
        assert_eq!(post.creator_id, Some(452674));
        assert_eq!(
            post.source.as_deref(),
            Some("https://www.pixiv.net/artworks/148271180")
        );
        assert_eq!(post.parent_id.as_deref(), Some("123455"));
        assert!(post.has_children);
        assert_eq!(post.created_at.as_deref(), Some("2026-08-11T01:02:03.000Z"));
        assert_eq!(post.rating, Rating::Safe);
        assert_eq!(
            variant_url(&post, MediaVariant::Full),
            post.full_url.as_deref()
        );
    }

    #[test]
    fn decodes_legacy_array_and_version_two_envelope() {
        let legacy = decode_posts(format!("[{IMAGE_JSON}]").as_bytes()).unwrap();
        let version_two =
            decode_posts(format!(r#"{{"posts":[{IMAGE_JSON}]}}"#).as_bytes()).unwrap();

        assert_eq!(legacy, version_two);
        assert_eq!(legacy.len(), 1);
    }

    #[test]
    fn missing_metadata_is_not_fabricated() {
        let posts = decode_posts(br#"[{"id":7,"tags":"artist -tag","rating":"x"}]"#).unwrap();

        assert_eq!(posts[0].width, None);
        assert_eq!(posts[0].full_url, None);
        assert_eq!(posts[0].rating, Rating::Unknown);
        assert_eq!(posts[0].tags, vec!["artist", "-tag"]);
    }

    #[test]
    fn numeric_created_at_is_normalized_for_moebooru_variants() {
        let posts = decode_posts(br#"[{"id":7,"created_at":1786390664}]"#).unwrap();

        assert_eq!(posts[0].created_at.as_deref(), Some("2026-08-10T19:37:44Z"));
    }

    #[test]
    fn malformed_post_payload_is_a_decode_error() {
        assert!(decode_posts(br#"{"unexpected":true}"#).is_err());
    }

    #[test]
    fn tag_query_preserves_negative_terms_and_adds_safe_filter() {
        assert_eq!(
            tag_expression("artist_name -sketch", ContentPolicy::SafeOnly).unwrap(),
            "artist_name -sketch rating:s"
        );
        assert_eq!(
            tag_query_params(
                "artist_name -sketch",
                ContentPolicy::AllowQuestionable,
                2,
                40
            )
            .unwrap(),
            vec![
                ("tags", "artist_name -sketch -rating:e".to_owned()),
                ("page", "2".to_owned()),
                ("limit", "40".to_owned()),
            ]
        );
    }

    #[test]
    fn tag_query_rejects_empty_or_invalid_page() {
        assert!(tag_expression(" ", ContentPolicy::SafeOnly).is_err());
        assert!(tag_query_params("tag", ContentPolicy::SafeOnly, 0, 40).is_err());
        assert!(tag_query_params("tag", ContentPolicy::SafeOnly, 1, 0).is_err());
    }

    #[test]
    fn popular_modes_map_to_paged_tag_expressions() {
        assert_eq!(
            popular_tag_expression(PopularPeriod::Day, "2026-08-10").unwrap(),
            "date:2026-08-10 order:score"
        );
        assert_eq!(
            popular_tag_expression(PopularPeriod::Week, "2026-08-10").unwrap(),
            "date:2026-08-10..2026-08-16 order:score"
        );
        assert_eq!(
            popular_tag_expression(PopularPeriod::Month, "2026-08-10").unwrap(),
            "date:2026-08-01..2026-08-31 order:score"
        );
        assert!(popular_tag_expression(PopularPeriod::Day, "2026-02-29").is_err());
    }

    #[test]
    fn tag_suggestions_map_category_metadata() {
        let suggestions = decode_tag_suggestions(
            br#"[{"name":"artist_name","count":12,"type":1,"ambiguous":false}]"#,
        )
        .unwrap();

        assert_eq!(suggestions[0].name, "artist_name");
        assert_eq!(suggestions[0].category, Some(TagCategory::Artist));
        assert_eq!(suggestions[0].post_count, Some(12));
        assert_eq!(suggestions[0].ambiguous, Some(false));
        assert!(suggestions[0].aliases.is_empty());
    }

    #[test]
    fn pools_map_to_site_neutral_records() {
        let pools =
            decode_pools(br#"[{"id":12,"name":"art book","is_public":true,"post_count":4}]"#)
                .unwrap();

        assert_eq!(pools[0].site.as_str(), SITE_ID);
        assert_eq!(pools[0].id, "12");
        assert_eq!(pools[0].name, "art book");
        assert_eq!(pools[0].post_count, 4);
        assert!(pools[0].public);
    }

    #[test]
    fn pool_query_is_a_safe_exact_tag_expression() {
        assert_eq!(pool_posts_expression("42").unwrap(), "pool:42");
        assert!(pool_posts_expression("42 order:score").is_err());
    }

    #[test]
    fn pool_zip_endpoint_is_site_owned_and_validated() {
        assert_eq!(
            pool_zip_endpoint("https://yande.re/post.json", "42").unwrap(),
            "https://yande.re/pool/zip/42"
        );
        assert!(pool_zip_endpoint("https://yande.re/post.json", "42/x").is_err());
    }

    #[test]
    fn query_mapping_keeps_search_and_popular_pagination_distinct() {
        let search = PostQueryRequest {
            query: dreamland_core::ReplayableQuery {
                source: DiscoverySource::Search {
                    expression: "artist_name -sketch".to_owned(),
                },
                content_policy: ContentPolicy::SafeOnly,
            },
            pagination: PaginationRequest::Page {
                number: 2,
                page_size: 40,
            },
        };
        let (_, search_params, _, _) =
            request_parts("https://yande.re/post.json", &search).unwrap();
        assert_eq!(
            search_params,
            vec![
                ("tags", "artist_name -sketch rating:s".to_owned()),
                ("page", "2".to_owned()),
                ("limit", "40".to_owned()),
            ]
        );

        let popular = PostQueryRequest {
            query: dreamland_core::ReplayableQuery {
                source: DiscoverySource::Feed {
                    kind: FeedKind::Popular {
                        period: PopularPeriod::Month,
                        anchor_date: "2026-08-10".to_owned(),
                    },
                },
                content_policy: ContentPolicy::SafeOnly,
            },
            pagination: PaginationRequest::Page {
                number: 2,
                page_size: 40,
            },
        };
        let (endpoint, params, _, fixed_window) =
            request_parts("https://yande.re/post.json", &popular).unwrap();
        assert_eq!(endpoint, "https://yande.re/post.json");
        assert_eq!(
            params,
            vec![
                (
                    "tags",
                    "date:2026-08-01..2026-08-31 order:score rating:s".to_owned()
                ),
                ("page", "2".to_owned()),
                ("limit", "40".to_owned()),
            ]
        );
        assert!(!fixed_window);
        assert!(request_parts(
            "https://yande.re/post.json",
            &PostQueryRequest {
                pagination: PaginationRequest::FixedWindow,
                ..popular
            }
        )
        .is_err());
    }

    #[test]
    fn http_statuses_map_to_stable_site_errors() {
        assert_eq!(
            map_http_status(reqwest::StatusCode::TOO_MANY_REQUESTS).code,
            SiteErrorCode::RateLimited
        );
        assert!(!map_http_status(reqwest::StatusCode::UNAUTHORIZED).retryable);
        assert!(map_http_status(reqwest::StatusCode::SERVICE_UNAVAILABLE).retryable);
    }

    #[tokio::test]
    async fn yande_client_identifies_dreamland() {
        use std::io::{Read, Write};
        use std::net::TcpListener;
        use std::thread;

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = Vec::new();
            let mut buffer = [0_u8; 4096];
            loop {
                let count = stream.read(&mut buffer).unwrap();
                request.extend_from_slice(&buffer[..count]);
                if request.windows(4).any(|window| window == b"\r\n\r\n") {
                    break;
                }
            }
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\n[]")
                .unwrap();
            request
        });
        let response = build_client(&NetworkPolicy {
            proxy: ProxyMode::Direct,
            ..NetworkPolicy::default()
        })
        .unwrap()
        .get(format!("http://{address}/post.json"))
        .send()
        .await
        .unwrap();
        assert!(response.status().is_success());
        let request = server.join().unwrap();
        assert!(request
            .windows(b"user-agent: dreamland/".len())
            .any(|window| { window.eq_ignore_ascii_case(b"user-agent: dreamland/") }));
    }

    #[test]
    fn default_config_is_loaded_from_the_bundled_toml_not_hardcoded() {
        assert_eq!(default_config().api_url, "https://yande.re/post.json");
    }

    #[test]
    fn browser_post_url_is_site_owned_and_validated() {
        let config = default_config();
        assert_eq!(
            browser_post_url(&config.browser_url, "407162").unwrap(),
            "https://yande.re/post/show/407162"
        );
        assert!(browser_post_url(&config.browser_url, "not-a-number").is_err());
    }
}
