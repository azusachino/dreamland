use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderId(String);

impl ProviderId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PostRef {
    pub provider: ProviderId,
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderCapabilities {
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
pub struct ProviderDescriptor {
    pub id: ProviderId,
    pub name: String,
    pub capabilities: ProviderCapabilities,
}
