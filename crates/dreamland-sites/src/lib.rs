use anyhow::{bail, Result};
use dreamland_core::{
    MediaVariant, NetworkPolicy, Post, PostQueryRequest, SitePage, TagSuggestion,
    TagSuggestionRequest,
};

pub use dreamland_core::SiteDescriptor;

pub const DEFAULT_SITE_ID: &str = dreamland_site_yandere::SITE_ID;

pub fn default_api_url(site_id: &str) -> Option<String> {
    (site_id == DEFAULT_SITE_ID).then(|| dreamland_site_yandere::default_config().api_url)
}

pub async fn query_posts(
    site_id: &str,
    api_url: &str,
    request: &PostQueryRequest,
    network: &NetworkPolicy,
) -> Result<SitePage> {
    if site_id != DEFAULT_SITE_ID {
        bail!("site '{site_id}' has no active post query adapter");
    }
    dreamland_site_yandere::query_posts(api_url, request, network).await
}

pub async fn suggest_tags(
    site_id: &str,
    api_url: &str,
    request: &TagSuggestionRequest,
    network: &NetworkPolicy,
) -> Result<Vec<TagSuggestion>> {
    if site_id != DEFAULT_SITE_ID {
        bail!("site '{site_id}' has no active tag suggestion adapter");
    }
    let endpoint = dreamland_site_yandere::tag_endpoint(api_url)?;
    dreamland_site_yandere::fetch_tag_suggestions(&endpoint, request, network).await
}

pub fn resolve_media_url<'a>(
    site_id: &str,
    post: &'a Post,
    variant: MediaVariant,
) -> Option<&'a str> {
    (site_id == DEFAULT_SITE_ID).then(|| dreamland_site_yandere::variant_url(post, variant))?
}

pub fn descriptors() -> Vec<SiteDescriptor> {
    vec![dreamland_site_yandere::descriptor()]
}

pub fn skeleton_descriptors() -> Vec<SiteDescriptor> {
    vec![
        dreamland_site_pixiv::descriptor(),
        dreamland_site_twitter::descriptor(),
    ]
}

/// The gate `src-tauri` must check before dispatching a browse/download
/// command to a site adapter. With a single active site this looks
/// redundant, but it is what stops the registry from being purely
/// decorative once a second site is wired in -- adding a descriptor here
/// does nothing on its own; a command still has to consult this first.
pub fn is_active_browse_site(site_id: &str) -> bool {
    descriptors()
        .iter()
        .any(|site| site.id.as_str() == site_id && site.capabilities.browse)
}

#[cfg(test)]
mod tests {
    use super::{default_api_url, descriptors, is_active_browse_site};

    #[test]
    fn registry_contains_yandere() {
        let sites = descriptors();

        assert_eq!(sites.len(), 1);
        assert_eq!(sites[0].id.as_str(), "yandere");
    }

    #[test]
    fn skeletons_are_not_active_sites() {
        let sites = super::skeleton_descriptors();

        assert_eq!(sites.len(), 2);
        assert!(sites.iter().all(|site| {
            !site.capabilities.browse
                && !site.capabilities.post_search
                && !site.capabilities.post_lookup
        }));
    }

    #[test]
    fn only_registered_browse_capable_sites_pass_the_gate() {
        assert!(is_active_browse_site("yandere"));
        assert!(!is_active_browse_site("pixiv"));
        assert!(!is_active_browse_site("does-not-exist"));
    }

    #[test]
    fn only_active_sites_have_default_runtime_configuration() {
        assert_eq!(
            default_api_url("yandere").as_deref(),
            Some("https://yande.re/post.json")
        );
        assert!(default_api_url("pixiv").is_none());
    }
}
