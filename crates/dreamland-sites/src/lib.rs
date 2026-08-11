use anyhow::{bail, Result};
use dreamland_core::{
    ContentPolicy, MediaVariant, NetworkPolicy, PoolPage, Post, PostQueryRequest, SitePage,
    TagSuggestion, TagSuggestionRequest,
};

pub use dreamland_core::SiteDescriptor;

pub const DEFAULT_SITE_ID: &str = dreamland_site_yandere::SITE_ID;

pub async fn query_posts(
    site_id: &str,
    request: &PostQueryRequest,
    network: &NetworkPolicy,
) -> Result<SitePage> {
    match site_id {
        dreamland_site_yandere::SITE_ID => {
            let config = dreamland_site_yandere::default_config();
            dreamland_site_yandere::query_posts(&config.api_url, request, network).await
        }
        dreamland_site_konachan::SITE_ID => {
            let config = dreamland_site_konachan::default_config();
            dreamland_site_konachan::query_posts(&config.api_url, request, network).await
        }
        _ => bail!("site '{site_id}' has no active post query adapter"),
    }
}

pub async fn lookup_post(site_id: &str, post_id: &str, network: &NetworkPolicy) -> Result<Post> {
    match site_id {
        dreamland_site_konachan::SITE_ID => {
            let config = dreamland_site_konachan::default_config();
            dreamland_site_konachan::lookup_post(&config.api_url, post_id, network).await
        }
        _ => bail!("site '{site_id}' has no active post lookup adapter"),
    }
}

pub async fn suggest_tags(
    site_id: &str,
    request: &TagSuggestionRequest,
    network: &NetworkPolicy,
) -> Result<Vec<TagSuggestion>> {
    match site_id {
        dreamland_site_yandere::SITE_ID => {
            let config = dreamland_site_yandere::default_config();
            let endpoint = dreamland_site_yandere::tag_endpoint(&config.api_url)?;
            dreamland_site_yandere::fetch_tag_suggestions(&endpoint, request, network).await
        }
        dreamland_site_konachan::SITE_ID => {
            let config = dreamland_site_konachan::default_config();
            let endpoint = dreamland_site_konachan::tag_endpoint(&config.api_url)?;
            dreamland_site_konachan::fetch_tag_suggestions(&endpoint, request, network).await
        }
        _ => bail!("site '{site_id}' has no active tag suggestion adapter"),
    }
}

pub async fn list_pools(
    site_id: &str,
    page: u32,
    page_size: u16,
    network: &NetworkPolicy,
) -> Result<PoolPage> {
    match site_id {
        dreamland_site_yandere::SITE_ID => {
            let config = dreamland_site_yandere::default_config();
            dreamland_site_yandere::fetch_pools(&config.api_url, page, page_size, network).await
        }
        dreamland_site_konachan::SITE_ID => {
            let config = dreamland_site_konachan::default_config();
            dreamland_site_konachan::fetch_pools(&config.api_url, page, page_size, network).await
        }
        _ => bail!("site '{site_id}' has no active pool adapter"),
    }
}

pub async fn query_pool_posts(
    site_id: &str,
    pool_id: &str,
    content_policy: ContentPolicy,
    page: u32,
    page_size: u16,
    network: &NetworkPolicy,
) -> Result<SitePage> {
    match site_id {
        dreamland_site_yandere::SITE_ID => {
            let config = dreamland_site_yandere::default_config();
            dreamland_site_yandere::query_pool_posts(
                &config.api_url,
                pool_id,
                content_policy,
                page,
                page_size,
                network,
            )
            .await
        }
        dreamland_site_konachan::SITE_ID => {
            let config = dreamland_site_konachan::default_config();
            dreamland_site_konachan::query_pool_posts(
                &config.api_url,
                pool_id,
                content_policy,
                page,
                page_size,
                network,
            )
            .await
        }
        _ => bail!("site '{site_id}' has no active pool adapter"),
    }
}

pub fn pool_zip_url(site_id: &str, pool_id: &str) -> Result<String> {
    if site_id != DEFAULT_SITE_ID {
        bail!("site '{site_id}' has no active pool archive adapter");
    }
    let config = dreamland_site_yandere::default_config();
    dreamland_site_yandere::pool_zip_endpoint(&config.api_url, pool_id)
}

pub async fn set_favorite(
    site_id: &str,
    post_id: &str,
    favorite: bool,
    cookie_header: &str,
    network: &NetworkPolicy,
) -> Result<()> {
    if site_id != DEFAULT_SITE_ID {
        bail!("site '{site_id}' has no active favorite adapter");
    }
    let config = dreamland_site_yandere::default_config();
    dreamland_site_yandere::set_favorite(&config.api_url, post_id, favorite, cookie_header, network)
        .await
}

pub async fn current_user(
    site_id: &str,
    user_id: &str,
    cookie_header: &str,
    network: &NetworkPolicy,
) -> Result<String> {
    if site_id != DEFAULT_SITE_ID {
        bail!("site '{site_id}' has no active auth adapter");
    }
    let config = dreamland_site_yandere::default_config();
    dreamland_site_yandere::current_user(&config.api_url, user_id, cookie_header, network).await
}

pub async fn list_favorites(
    site_id: &str,
    username: &str,
    cookie_header: &str,
    content_policy: ContentPolicy,
    page: u32,
    page_size: u16,
    network: &NetworkPolicy,
) -> Result<SitePage> {
    if site_id != DEFAULT_SITE_ID {
        bail!("site '{site_id}' has no active favorite-list adapter");
    }
    let config = dreamland_site_yandere::default_config();
    dreamland_site_yandere::list_favorites(
        &config.api_url,
        username,
        cookie_header,
        content_policy,
        page,
        page_size,
        network,
    )
    .await
}

pub fn resolve_media_url<'a>(
    site_id: &str,
    post: &'a Post,
    variant: MediaVariant,
) -> Option<&'a str> {
    match site_id {
        dreamland_site_yandere::SITE_ID => dreamland_site_yandere::variant_url(post, variant),
        dreamland_site_konachan::SITE_ID => dreamland_site_konachan::variant_url(post, variant),
        _ => None,
    }
}

pub fn browser_url(site_id: &str) -> Result<String> {
    match site_id {
        dreamland_site_yandere::SITE_ID => Ok(dreamland_site_yandere::default_config().browser_url),
        dreamland_site_konachan::SITE_ID => {
            Ok(dreamland_site_konachan::default_config().browser_url)
        }
        _ => bail!("site '{site_id}' has no active browser route"),
    }
}

pub fn descriptors() -> Vec<SiteDescriptor> {
    vec![
        dreamland_site_yandere::descriptor(),
        dreamland_site_konachan::descriptor(),
    ]
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
    use super::{browser_url, descriptors, is_active_browse_site};

    #[test]
    fn registry_contains_yandere() {
        let sites = descriptors();

        assert_eq!(sites.len(), 2);
        assert_eq!(sites[0].id.as_str(), "yandere");
        assert_eq!(sites[1].id.as_str(), "konachan");
        assert!(sites[1].capabilities.collections);
        assert!(sites[1].capabilities.post_lookup);
        assert!(!sites[1].capabilities.collection_downloads);
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
        assert!(is_active_browse_site("konachan"));
        assert!(!is_active_browse_site("pixiv"));
        assert!(!is_active_browse_site("does-not-exist"));
    }

    #[test]
    fn browser_routes_are_owned_by_the_site_adapters() {
        assert_eq!(browser_url("yandere").unwrap(), "https://yande.re/post");
        assert_eq!(
            browser_url("konachan").unwrap(),
            "https://konachan.com/post"
        );
        assert!(browser_url("pixiv").is_err());
    }
}
