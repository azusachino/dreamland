use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SiteId(String);

impl SiteId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Runtime-owned authentication material for one site.
#[derive(Clone)]
pub struct SiteSession {
    site: SiteId,
    user_id: String,
    cookie_header: String,
}

#[derive(Clone)]
pub struct BrowserCookie {
    pub name: String,
    pub value: String,
}

impl SiteSession {
    pub fn new(site: SiteId, user_id: String, cookie_header: String) -> Self {
        Self {
            site,
            user_id,
            cookie_header,
        }
    }

    pub fn site(&self) -> &SiteId {
        &self.site
    }

    pub fn user_id(&self) -> &str {
        &self.user_id
    }

    pub fn cookie_header(&self) -> &str {
        &self.cookie_header
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PostRef {
    pub site: SiteId,
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SiteCapabilities {
    pub browse: bool,
    pub post_search: bool,
    pub tag_search: bool,
    pub related_tags: bool,
    pub tag_query: bool,
    pub post_lookup: bool,
    pub similar_search: bool,
    pub page_numbers: bool,
    pub cursors: bool,
    pub multiple_download_variants: bool,
    pub safe_content_only: bool,
    pub authentication: bool,
    pub remote_favorites: bool,
    pub favorite_list: bool,
    pub collections: bool,
    pub collection_downloads: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PostQueryRequest {
    pub query: ReplayableQuery,
    pub pagination: PaginationRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReplayableQuery {
    pub source: DiscoverySource,
    pub content_policy: ContentPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SavedQuery {
    pub id: String,
    pub site: SiteId,
    pub name: String,
    pub query: ReplayableQuery,
    pub pinned: bool,
    pub position: u32,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TagSuggestionRequest {
    pub query: String,
    pub limit: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RelatedTagRequest {
    pub tags: Vec<String>,
    pub limit: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TagSuggestion {
    pub name: String,
    pub category: Option<TagCategory>,
    pub post_count: Option<u64>,
    pub ambiguous: Option<bool>,
    pub aliases: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RelatedTag {
    pub name: String,
    pub post_count: Option<u64>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TagCategory {
    General,
    Artist,
    Copyright,
    Character,
    Metadata,
    Unknown(u8),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DiscoverySource {
    Browse,
    Search { expression: String },
    Feed { kind: FeedKind },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FeedKind {
    Latest,
    Popular {
        period: PopularPeriod,
        anchor_date: String,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PopularPeriod {
    Day,
    Week,
    Month,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PaginationRequest {
    First { page_size: u16 },
    Page { number: u32, page_size: u16 },
    FixedWindow,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum ContentPolicy {
    #[default]
    SafeOnly,
    AllowQuestionable,
    AllowExplicit,
    ExplicitOnly,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SiteErrorCode {
    InvalidRequest,
    UnsupportedCapability,
    DecodeFailed,
    RateLimited,
    AuthRequired,
    NetworkFailed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SiteError {
    pub code: SiteErrorCode,
    pub message: String,
    pub retryable: bool,
}

impl SiteError {
    pub fn new(code: SiteErrorCode, message: impl Into<String>, retryable: bool) -> Self {
        Self {
            code,
            message: message.into(),
            retryable,
        }
    }
}

impl std::fmt::Display for SiteError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{:?}: {}", self.code, self.message)
    }
}

impl std::error::Error for SiteError {}

pub type SiteFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, SiteError>> + Send + 'a>>;

pub trait PostQueryCapability: Send + Sync {
    fn query_posts<'a>(
        &'a self,
        request: &'a PostQueryRequest,
        network: &'a NetworkPolicy,
    ) -> SiteFuture<'a, SitePage>;
}

pub trait TagSuggestionCapability: Send + Sync {
    fn suggest_tags<'a>(
        &'a self,
        request: &'a TagSuggestionRequest,
        network: &'a NetworkPolicy,
    ) -> SiteFuture<'a, Vec<TagSuggestion>>;
}

pub trait RelatedTagCapability: Send + Sync {
    fn related_tags<'a>(
        &'a self,
        request: &'a RelatedTagRequest,
        network: &'a NetworkPolicy,
    ) -> SiteFuture<'a, Vec<RelatedTag>>;
}

pub trait PostLookupCapability: Send + Sync {
    fn lookup_post<'a>(
        &'a self,
        post_id: &'a str,
        content_policy: ContentPolicy,
        network: &'a NetworkPolicy,
    ) -> SiteFuture<'a, Post>;
}

pub trait CollectionCapability: Send + Sync {
    fn list_pools<'a>(
        &'a self,
        query: &'a str,
        page: u32,
        page_size: u16,
        network: &'a NetworkPolicy,
    ) -> SiteFuture<'a, PoolPage>;

    fn query_pool_posts<'a>(
        &'a self,
        pool_id: &'a str,
        content_policy: ContentPolicy,
        page: u32,
        page_size: u16,
        network: &'a NetworkPolicy,
    ) -> SiteFuture<'a, SitePage>;
}

pub trait CollectionDownloadCapability: Send + Sync {
    fn pool_zip_url(&self, pool_id: &str) -> Result<String, SiteError>;
}

pub trait RemoteFavoriteCapability: Send + Sync {
    fn set_favorite<'a>(
        &'a self,
        post_id: &'a str,
        favorite: bool,
        session: &'a SiteSession,
        network: &'a NetworkPolicy,
    ) -> SiteFuture<'a, ()>;
}

pub trait RemoteFavoriteListCapability: Send + Sync {
    fn list_favorites<'a>(
        &'a self,
        session: &'a SiteSession,
        content_policy: ContentPolicy,
        page: u32,
        page_size: u16,
        network: &'a NetworkPolicy,
    ) -> SiteFuture<'a, SitePage>;
}

pub trait CurrentUserCapability: Send + Sync {
    fn current_user<'a>(
        &'a self,
        session: &'a SiteSession,
        network: &'a NetworkPolicy,
    ) -> SiteFuture<'a, String>;
}

pub trait AuthenticationCapability: Send + Sync {
    fn session_from_cookies(
        &self,
        cookies: &[BrowserCookie],
    ) -> Result<Option<SiteSession>, SiteError>;

    fn login_url(&self) -> Result<String, SiteError>;
}

pub trait MediaResolutionCapability: Send + Sync {
    fn resolve_media_url<'a>(&'a self, post: &'a Post, variant: MediaVariant) -> Option<&'a str>;
}

pub trait BrowserRoutesCapability: Send + Sync {
    fn browser_url(&self) -> Result<String, SiteError>;
    fn browser_post_url(&self, post_id: &str) -> Result<String, SiteError>;
    fn browser_similar_url(&self) -> Result<String, SiteError>;
}

pub trait SiteAdapter: Send + Sync {
    fn descriptor(&self) -> &SiteDescriptor;
    fn post_query(&self) -> &dyn PostQueryCapability;
    fn tag_suggestions(&self) -> Option<&dyn TagSuggestionCapability> {
        None
    }
    fn related_tags(&self) -> Option<&dyn RelatedTagCapability> {
        None
    }
    fn post_lookup(&self) -> Option<&dyn PostLookupCapability> {
        None
    }
    fn collections(&self) -> Option<&dyn CollectionCapability> {
        None
    }
    fn collection_download(&self) -> Option<&dyn CollectionDownloadCapability> {
        None
    }
    fn remote_favorites(&self) -> Option<&dyn RemoteFavoriteCapability> {
        None
    }
    fn remote_favorite_list(&self) -> Option<&dyn RemoteFavoriteListCapability> {
        None
    }
    fn current_user(&self) -> Option<&dyn CurrentUserCapability> {
        None
    }
    fn authentication(&self) -> Option<&dyn AuthenticationCapability> {
        None
    }
    fn media_resolution(&self) -> &dyn MediaResolutionCapability;
    fn browser_routes(&self) -> &dyn BrowserRoutesCapability;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct NetworkPolicy {
    pub proxy: ProxyMode,
    pub max_retries: u8,
    pub retry_delay_ms: u64,
    pub max_retry_delay_ms: u64,
}

impl Default for NetworkPolicy {
    fn default() -> Self {
        Self {
            proxy: ProxyMode::Auto,
            max_retries: 2,
            retry_delay_ms: 500,
            max_retry_delay_ms: 8_000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProxyMode {
    Auto,
    Direct,
    Manual { url: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct QuerySessionId(String);

impl QuerySessionId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContinuationToken(String);

impl ContinuationToken {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Continuation {
    None,
    Next(ContinuationToken),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SitePage {
    pub posts: Vec<Post>,
    pub continuation: Continuation,
    pub total: Option<u64>,
    pub page_size: u16,
    pub session: Option<QuerySessionId>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Pool {
    pub site: SiteId,
    pub id: String,
    pub name: String,
    pub post_count: u32,
    pub public: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PoolPage {
    pub pools: Vec<Pool>,
    pub page: u32,
    pub page_size: u16,
    pub has_next: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SiteDescriptor {
    pub id: SiteId,
    pub name: String,
    pub capabilities: SiteCapabilities,
}

/// Site-neutral post shape crossing the Tauri boundary. Site adapters parse
/// their own wire format internally and map into this type; the frontend
/// never sees a site's raw response shape.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Post {
    pub post: PostRef,
    pub tags: Vec<String>,
    pub author: Option<String>,
    pub creator_id: Option<u64>,
    pub md5: Option<String>,
    pub source: Option<String>,
    pub parent_id: Option<String>,
    pub has_children: bool,
    pub created_at: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub rating: Rating,
    pub score: Option<i32>,
    pub preview_url: Option<String>,
    pub sample_url: Option<String>,
    pub full_url: Option<String>,
    pub file_size: Option<u64>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Rating {
    Safe,
    Questionable,
    Explicit,
    Unknown,
}

/// The variant a download command may request. The frontend selects one of
/// these against a `PostRef`; it never supplies a URL directly.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum MediaVariant {
    Preview,
    Sample,
    Full,
}
