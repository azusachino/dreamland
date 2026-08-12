use anyhow::{bail, Context, Result};
use dreamland_core::{
    AuthenticationCapability, BrowserCookie, BrowserRoutesCapability, CollectionCapability,
    CollectionDownloadCapability, ContentPolicy, CurrentUserCapability, DiscoverySource,
    MediaResolutionCapability, MediaVariant, NetworkPolicy, PaginationRequest, Pool, PoolPage,
    PopularPeriod, Post, PostLookupCapability, PostQueryCapability, PostQueryRequest, ProxyMode,
    RelatedTagCapability, RemoteFavoriteCapability, RemoteFavoriteListCapability, SiteAdapter,
    SiteCapabilities, SiteDescriptor, SiteError, SiteErrorCode, SiteFuture, SiteId, SitePage,
    SiteSession, TagSuggestion, TagSuggestionCapability, TagSuggestionRequest,
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

#[derive(Clone)]
pub struct Adapter {
    config: SiteDefaults,
    descriptor: SiteDescriptor,
}

impl Default for Adapter {
    fn default() -> Self {
        Self {
            config: default_config(),
            descriptor: descriptor(),
        }
    }
}

fn adapter_error(error: anyhow::Error) -> SiteError {
    if let Some(site_error) = error.downcast_ref::<SiteError>() {
        return site_error.clone();
    }
    if error.downcast_ref::<reqwest::Error>().is_some() {
        return SiteError::new(SiteErrorCode::NetworkFailed, error.to_string(), true);
    }
    dreamland_moe::map_error_message(&error.to_string(), SITE_ID)
}

fn parse_yande_user_id(value: &str) -> Option<String> {
    let id = value.split(';').next()?.trim().parse::<u64>().ok()?;
    (id > 0).then(|| id.to_string())
}

impl PostQueryCapability for Adapter {
    fn query_posts<'a>(
        &'a self,
        request: &'a PostQueryRequest,
        network: &'a NetworkPolicy,
    ) -> SiteFuture<'a, SitePage> {
        Box::pin(async move {
            query_posts(&self.config.api_url, request, network)
                .await
                .map_err(adapter_error)
        })
    }
}

impl TagSuggestionCapability for Adapter {
    fn suggest_tags<'a>(
        &'a self,
        request: &'a TagSuggestionRequest,
        network: &'a NetworkPolicy,
    ) -> SiteFuture<'a, Vec<TagSuggestion>> {
        Box::pin(async move {
            let endpoint = tag_endpoint(&self.config.api_url).map_err(adapter_error)?;
            fetch_tag_suggestions(&endpoint, request, network)
                .await
                .map_err(adapter_error)
        })
    }
}

impl RelatedTagCapability for Adapter {
    fn related_tags<'a>(
        &'a self,
        request: &'a dreamland_core::RelatedTagRequest,
        network: &'a NetworkPolicy,
    ) -> SiteFuture<'a, Vec<dreamland_core::RelatedTag>> {
        Box::pin(async move {
            let endpoint = related_tag_endpoint(&self.config.api_url).map_err(adapter_error)?;
            fetch_related_tags(&endpoint, request, network)
                .await
                .map_err(adapter_error)
        })
    }
}

impl CollectionCapability for Adapter {
    fn list_pools<'a>(
        &'a self,
        query: &'a str,
        page: u32,
        page_size: u16,
        network: &'a NetworkPolicy,
    ) -> SiteFuture<'a, PoolPage> {
        Box::pin(async move {
            fetch_pools(&self.config.api_url, query, page, page_size, network)
                .await
                .map_err(adapter_error)
        })
    }

    fn query_pool_posts<'a>(
        &'a self,
        pool_id: &'a str,
        content_policy: ContentPolicy,
        page: u32,
        page_size: u16,
        network: &'a NetworkPolicy,
    ) -> SiteFuture<'a, SitePage> {
        Box::pin(async move {
            query_pool_posts(
                &self.config.api_url,
                pool_id,
                content_policy,
                page,
                page_size,
                network,
            )
            .await
            .map_err(adapter_error)
        })
    }
}

impl CollectionDownloadCapability for Adapter {
    fn pool_zip_url(&self, pool_id: &str) -> Result<String, SiteError> {
        pool_zip_endpoint(&self.config.api_url, pool_id).map_err(adapter_error)
    }
}

impl RemoteFavoriteCapability for Adapter {
    fn set_favorite<'a>(
        &'a self,
        post_id: &'a str,
        favorite: bool,
        session: &'a SiteSession,
        network: &'a NetworkPolicy,
    ) -> SiteFuture<'a, ()> {
        Box::pin(async move {
            set_favorite(
                &self.config.api_url,
                post_id,
                favorite,
                session.cookie_header(),
                network,
            )
            .await
            .map_err(adapter_error)
        })
    }
}

impl RemoteFavoriteListCapability for Adapter {
    fn list_favorites<'a>(
        &'a self,
        session: &'a SiteSession,
        content_policy: ContentPolicy,
        page: u32,
        page_size: u16,
        network: &'a NetworkPolicy,
    ) -> SiteFuture<'a, SitePage> {
        Box::pin(async move {
            list_favorites(
                &self.config.api_url,
                session.user_id(),
                session.cookie_header(),
                content_policy,
                page,
                page_size,
                network,
            )
            .await
            .map_err(adapter_error)
        })
    }
}

impl CurrentUserCapability for Adapter {
    fn current_user<'a>(
        &'a self,
        session: &'a SiteSession,
        network: &'a NetworkPolicy,
    ) -> SiteFuture<'a, String> {
        Box::pin(async move {
            current_user(
                &self.config.api_url,
                session.user_id(),
                session.cookie_header(),
                network,
            )
            .await
            .map_err(adapter_error)
        })
    }
}

impl AuthenticationCapability for Adapter {
    fn session_from_cookies(
        &self,
        cookies: &[BrowserCookie],
    ) -> Result<Option<SiteSession>, SiteError> {
        let user_id = cookies
            .iter()
            .find(|cookie| cookie.name == "user_id")
            .and_then(|cookie| parse_yande_user_id(&cookie.value))
            .or_else(|| {
                cookies
                    .iter()
                    .find(|cookie| cookie.name == "user_info")
                    .and_then(|cookie| parse_yande_user_id(&cookie.value))
            });
        let Some(user_id) = user_id else {
            return Ok(None);
        };
        let cookie_header = cookies
            .iter()
            .map(|cookie| format!("{}={}", cookie.name, cookie.value))
            .collect::<Vec<_>>()
            .join("; ");
        Ok(Some(SiteSession::new(
            SiteId::new(SITE_ID),
            user_id,
            cookie_header,
        )))
    }

    fn login_url(&self) -> Result<String, SiteError> {
        Ok(format!("{}/user/login", self.config.browser_url))
    }
}

impl MediaResolutionCapability for Adapter {
    fn resolve_media_url<'a>(&'a self, post: &'a Post, variant: MediaVariant) -> Option<&'a str> {
        variant_url(post, variant)
    }
}

impl BrowserRoutesCapability for Adapter {
    fn browser_url(&self) -> Result<String, SiteError> {
        Ok(self.config.browser_url.clone())
    }

    fn browser_post_url(&self, post_id: &str) -> Result<String, SiteError> {
        browser_post_url(&self.config.browser_url, post_id).map_err(adapter_error)
    }

    fn browser_similar_url(&self) -> Result<String, SiteError> {
        Err(SiteError::new(
            SiteErrorCode::UnsupportedCapability,
            "yandere does not provide a similar-search browser route",
            false,
        ))
    }
}

impl SiteAdapter for Adapter {
    fn descriptor(&self) -> &SiteDescriptor {
        &self.descriptor
    }

    fn post_query(&self) -> &dyn PostQueryCapability {
        self
    }

    fn tag_suggestions(&self) -> Option<&dyn TagSuggestionCapability> {
        Some(self)
    }

    fn related_tags(&self) -> Option<&dyn RelatedTagCapability> {
        Some(self)
    }

    fn post_lookup(&self) -> Option<&dyn PostLookupCapability> {
        None
    }

    fn collections(&self) -> Option<&dyn CollectionCapability> {
        Some(self)
    }

    fn collection_download(&self) -> Option<&dyn CollectionDownloadCapability> {
        Some(self)
    }

    fn remote_favorites(&self) -> Option<&dyn RemoteFavoriteCapability> {
        Some(self)
    }

    fn remote_favorite_list(&self) -> Option<&dyn RemoteFavoriteListCapability> {
        Some(self)
    }

    fn current_user(&self) -> Option<&dyn CurrentUserCapability> {
        Some(self)
    }

    fn authentication(&self) -> Option<&dyn AuthenticationCapability> {
        Some(self)
    }

    fn media_resolution(&self) -> &dyn MediaResolutionCapability {
        self
    }

    fn browser_routes(&self) -> &dyn BrowserRoutesCapability {
        self
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
            return Err(anyhow::Error::new(error));
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
fn request_parts(api_url: &str, request: &PostQueryRequest) -> Result<dreamland_moe::RequestParts> {
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
mod tests;
