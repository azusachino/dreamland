use dreamland_core::{SiteCapabilities, SiteDescriptor, SiteId};

pub const SITE_ID: &str = "pixiv";

/// Descriptor-only skeleton. No Pixiv operation is active until its adapter
/// implements the advertised capability traits and receives fixture coverage.
pub fn descriptor() -> SiteDescriptor {
    SiteDescriptor {
        id: SiteId::new(SITE_ID),
        name: "Pixiv".to_owned(),
        capabilities: SiteCapabilities {
            browse: false,
            post_search: false,
            tag_search: false,
            tag_query: false,
            post_lookup: false,
            page_numbers: false,
            cursors: false,
            multiple_download_variants: false,
        },
    }
}
