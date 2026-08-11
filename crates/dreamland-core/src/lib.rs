use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ContentPolicy {
    SafeOnly,
    AllowQuestionable,
    AllowExplicit,
    ExplicitOnly,
}

impl Default for ContentPolicy {
    fn default() -> Self {
        Self::SafeOnly
    }
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
