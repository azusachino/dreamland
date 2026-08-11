use dreamland_core::{SiteCapabilities, SiteDescriptor, SiteId};

pub const SITE_ID: &str = "twitter";

/// Descriptor-only skeleton. No Twitter operation is active until its adapter
/// implements the relevant capability traits and receives fixture coverage.
pub fn descriptor() -> SiteDescriptor {
    SiteDescriptor {
        id: SiteId::new(SITE_ID),
        name: "twitter".to_owned(),
        capabilities: SiteCapabilities {
            browse: false,
            post_search: false,
            tag_search: false,
            related_tags: false,
            tag_query: false,
            post_lookup: false,
            similar_search: false,
            page_numbers: false,
            cursors: false,
            multiple_download_variants: false,
            safe_content_only: false,
            authentication: false,
            remote_favorites: false,
            favorite_list: false,
            collections: false,
            collection_downloads: false,
        },
    }
}
