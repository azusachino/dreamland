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
    pub tag_query: bool,
    pub post_lookup: bool,
    pub page_numbers: bool,
    pub cursors: bool,
    pub multiple_download_variants: bool,
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
    pub width: u32,
    pub height: u32,
    pub rating: Rating,
    pub score: Option<i32>,
    pub preview_url: String,
    pub sample_url: String,
    pub full_url: String,
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
