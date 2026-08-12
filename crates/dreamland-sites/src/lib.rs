use anyhow::{anyhow, bail, Result};
use dreamland_core::{
    BrowserCookie, ContentPolicy, MediaVariant, NetworkPolicy, PoolPage, Post, PostQueryRequest,
    RelatedTag, RelatedTagRequest, SiteAdapter, SiteError, SiteErrorCode, SitePage, SiteSession,
    TagSuggestion, TagSuggestionRequest,
};
use std::collections::BTreeSet;

pub use dreamland_core::SiteDescriptor;

pub const DEFAULT_SITE_ID: &str = dreamland_site_yandere::SITE_ID;

pub struct SiteRegistry {
    active: Vec<Box<dyn SiteAdapter>>,
    skeletons: Vec<SiteDescriptor>,
}

impl Default for SiteRegistry {
    fn default() -> Self {
        Self::from_enabled([
            dreamland_site_yandere::SITE_ID,
            dreamland_site_konachan::SITE_ID,
        ])
    }
}

impl SiteRegistry {
    pub fn from_enabled<'a>(enabled: impl IntoIterator<Item = &'a str>) -> Self {
        let enabled: BTreeSet<&str> = enabled.into_iter().collect();
        Self {
            active: [
                (
                    dreamland_site_yandere::SITE_ID,
                    Box::new(dreamland_site_yandere::Adapter::default()) as Box<dyn SiteAdapter>,
                ),
                (
                    dreamland_site_konachan::SITE_ID,
                    Box::new(dreamland_site_konachan::Adapter::default()) as Box<dyn SiteAdapter>,
                ),
            ]
            .into_iter()
            .filter_map(|(site_id, adapter)| enabled.contains(site_id).then_some(adapter))
            .collect(),
            skeletons: vec![
                dreamland_site_pixiv::descriptor(),
                dreamland_site_twitter::descriptor(),
            ],
        }
    }
}

impl SiteRegistry {
    pub fn validate(&self) -> Result<()> {
        for adapter in &self.active {
            let descriptor = adapter.descriptor();
            if !descriptor.capabilities.browse {
                bail!(
                    "active site '{}' must support browse",
                    descriptor.id.as_str()
                );
            }
            if descriptor.capabilities.post_lookup != adapter.post_lookup().is_some() {
                bail!(
                    "site '{}' post_lookup descriptor does not match its adapter port",
                    descriptor.id.as_str()
                );
            }
            if descriptor.capabilities.related_tags != adapter.related_tags().is_some() {
                bail!(
                    "site '{}' related_tags descriptor does not match its adapter port",
                    descriptor.id.as_str()
                );
            }
            if descriptor.capabilities.collections != adapter.collections().is_some() {
                bail!(
                    "site '{}' collections descriptor does not match its adapter port",
                    descriptor.id.as_str()
                );
            }
            if descriptor.capabilities.collection_downloads
                != adapter.collection_download().is_some()
            {
                bail!(
                    "site '{}' collection_downloads descriptor does not match its adapter port",
                    descriptor.id.as_str()
                );
            }
            if descriptor.capabilities.remote_favorites != adapter.remote_favorites().is_some() {
                bail!(
                    "site '{}' remote_favorites descriptor does not match its adapter port",
                    descriptor.id.as_str()
                );
            }
            if descriptor.capabilities.favorite_list != adapter.remote_favorite_list().is_some() {
                bail!(
                    "site '{}' favorite_list descriptor does not match its adapter port",
                    descriptor.id.as_str()
                );
            }
            if descriptor.capabilities.authentication != adapter.current_user().is_some() {
                bail!(
                    "site '{}' authentication descriptor does not match its adapter port",
                    descriptor.id.as_str()
                );
            }
            if descriptor.capabilities.authentication != adapter.authentication().is_some() {
                bail!(
                    "site '{}' authentication descriptor does not match its auth port",
                    descriptor.id.as_str()
                );
            }
        }
        let mut ids = BTreeSet::new();
        for adapter in &self.active {
            let id = adapter.descriptor().id.as_str();
            if !ids.insert(id) {
                bail!("duplicate active site id: {id}");
            }
        }
        for skeleton in &self.skeletons {
            if !ids.insert(skeleton.id.as_str()) {
                bail!(
                    "active and skeleton site ids collide: {}",
                    skeleton.id.as_str()
                );
            }
        }
        Ok(())
    }

    pub fn site(&self, site_id: &str) -> Result<&dyn SiteAdapter> {
        self.active
            .iter()
            .find(|adapter| adapter.descriptor().id.as_str() == site_id)
            .map(|adapter| adapter.as_ref())
            .ok_or_else(|| anyhow!("site '{site_id}' is not an active site"))
    }

    pub fn descriptors(&self) -> Vec<SiteDescriptor> {
        self.active
            .iter()
            .map(|adapter| adapter.descriptor().clone())
            .collect()
    }

    pub fn skeleton_descriptors(&self) -> &[SiteDescriptor] {
        &self.skeletons
    }

    pub fn all_descriptors(&self) -> Vec<SiteDescriptor> {
        self.descriptors()
            .into_iter()
            .chain(self.skeletons.iter().cloned())
            .collect()
    }

    pub fn is_active_browse_site(&self, site_id: &str) -> bool {
        self.site(site_id)
            .map(|adapter| adapter.descriptor().capabilities.browse)
            .unwrap_or(false)
    }

    pub async fn query_posts(
        &self,
        site_id: &str,
        request: &PostQueryRequest,
        network: &NetworkPolicy,
    ) -> Result<SitePage> {
        self.site(site_id)?
            .post_query()
            .query_posts(request, network)
            .await
            .map_err(site_error)
    }

    pub async fn lookup_post(
        &self,
        site_id: &str,
        post_id: &str,
        content_policy: ContentPolicy,
        network: &NetworkPolicy,
    ) -> Result<Post> {
        let capability = self
            .site(site_id)?
            .post_lookup()
            .ok_or_else(|| unsupported(site_id, "post lookup"))?;
        capability
            .lookup_post(post_id, content_policy, network)
            .await
            .map_err(site_error)
    }

    pub async fn suggest_tags(
        &self,
        site_id: &str,
        request: &TagSuggestionRequest,
        network: &NetworkPolicy,
    ) -> Result<Vec<TagSuggestion>> {
        let capability = self
            .site(site_id)?
            .tag_suggestions()
            .ok_or_else(|| unsupported(site_id, "tag suggestions"))?;
        capability
            .suggest_tags(request, network)
            .await
            .map_err(site_error)
    }

    pub async fn related_tags(
        &self,
        site_id: &str,
        request: &RelatedTagRequest,
        network: &NetworkPolicy,
    ) -> Result<Vec<RelatedTag>> {
        let capability = self
            .site(site_id)?
            .related_tags()
            .ok_or_else(|| unsupported(site_id, "related tags"))?;
        capability
            .related_tags(request, network)
            .await
            .map_err(site_error)
    }

    pub async fn list_pools(
        &self,
        site_id: &str,
        query: &str,
        page: u32,
        page_size: u16,
        network: &NetworkPolicy,
    ) -> Result<PoolPage> {
        let capability = self
            .site(site_id)?
            .collections()
            .ok_or_else(|| unsupported(site_id, "collections"))?;
        capability
            .list_pools(query, page, page_size, network)
            .await
            .map_err(site_error)
    }

    pub async fn query_pool_posts(
        &self,
        site_id: &str,
        pool_id: &str,
        content_policy: ContentPolicy,
        page: u32,
        page_size: u16,
        network: &NetworkPolicy,
    ) -> Result<SitePage> {
        let capability = self
            .site(site_id)?
            .collections()
            .ok_or_else(|| unsupported(site_id, "collection posts"))?;
        capability
            .query_pool_posts(pool_id, content_policy, page, page_size, network)
            .await
            .map_err(site_error)
    }

    pub fn pool_zip_url(&self, site_id: &str, pool_id: &str) -> Result<String> {
        let capability = self
            .site(site_id)?
            .collection_download()
            .ok_or_else(|| unsupported(site_id, "pool archive download"))?;
        capability.pool_zip_url(pool_id).map_err(site_error)
    }

    pub async fn set_favorite(
        &self,
        site_id: &str,
        post_id: &str,
        favorite: bool,
        session: &SiteSession,
        network: &NetworkPolicy,
    ) -> Result<()> {
        ensure_session_site(site_id, session)?;
        let capability = self
            .site(site_id)?
            .remote_favorites()
            .ok_or_else(|| unsupported(site_id, "remote favorites"))?;
        capability
            .set_favorite(post_id, favorite, session, network)
            .await
            .map_err(site_error)
    }

    pub async fn current_user(
        &self,
        site_id: &str,
        session: &SiteSession,
        network: &NetworkPolicy,
    ) -> Result<String> {
        ensure_session_site(site_id, session)?;
        let capability = self
            .site(site_id)?
            .current_user()
            .ok_or_else(|| unsupported(site_id, "current-user lookup"))?;
        capability
            .current_user(session, network)
            .await
            .map_err(site_error)
    }

    pub async fn list_favorites(
        &self,
        site_id: &str,
        session: &SiteSession,
        content_policy: ContentPolicy,
        page: u32,
        page_size: u16,
        network: &NetworkPolicy,
    ) -> Result<SitePage> {
        ensure_session_site(site_id, session)?;
        let capability = self
            .site(site_id)?
            .remote_favorite_list()
            .ok_or_else(|| unsupported(site_id, "remote favorite list"))?;
        capability
            .list_favorites(session, content_policy, page, page_size, network)
            .await
            .map_err(site_error)
    }

    pub fn auth_session(
        &self,
        site_id: &str,
        cookies: &[BrowserCookie],
    ) -> Result<Option<SiteSession>> {
        self.site(site_id)?
            .authentication()
            .ok_or_else(|| unsupported(site_id, "authentication"))?
            .session_from_cookies(cookies)
            .map_err(site_error)
    }

    pub fn auth_login_url(&self, site_id: &str) -> Result<String> {
        self.site(site_id)?
            .authentication()
            .ok_or_else(|| unsupported(site_id, "authentication"))?
            .login_url()
            .map_err(site_error)
    }

    pub fn resolve_media_url(
        &self,
        site_id: &str,
        post: &Post,
        variant: MediaVariant,
    ) -> Option<String> {
        self.site(site_id)
            .ok()
            .and_then(|adapter| adapter.media_resolution().resolve_media_url(post, variant))
            .map(str::to_owned)
    }

    pub fn browser_url(&self, site_id: &str) -> Result<String> {
        self.site(site_id)?
            .browser_routes()
            .browser_url()
            .map_err(site_error)
    }

    pub fn browser_post_url(&self, site_id: &str, post_id: &str) -> Result<String> {
        self.site(site_id)?
            .browser_routes()
            .browser_post_url(post_id)
            .map_err(site_error)
    }

    pub fn browser_similar_url(&self, site_id: &str) -> Result<String> {
        self.site(site_id)?
            .browser_routes()
            .browser_similar_url()
            .map_err(site_error)
    }
}

fn unsupported(site_id: &str, capability: &str) -> anyhow::Error {
    anyhow::Error::new(SiteError::new(
        SiteErrorCode::UnsupportedCapability,
        format!("site '{site_id}' does not support {capability}"),
        false,
    ))
}

fn ensure_session_site(site_id: &str, session: &SiteSession) -> Result<()> {
    if session.site().as_str() != site_id {
        return Err(anyhow::Error::new(SiteError::new(
            SiteErrorCode::InvalidRequest,
            format!(
                "session belongs to '{}' but was used for '{site_id}'",
                session.site().as_str()
            ),
            false,
        )));
    }
    Ok(())
}

fn site_error(error: SiteError) -> anyhow::Error {
    anyhow::Error::new(error)
}

#[cfg(test)]
mod tests {
    use super::{SiteError, SiteErrorCode, SiteRegistry};

    #[test]
    fn registry_contains_only_real_active_adapters() {
        let registry = SiteRegistry::default();
        registry.validate().unwrap();
        assert_eq!(
            registry
                .descriptors()
                .iter()
                .map(|descriptor| descriptor.id.as_str())
                .collect::<Vec<_>>(),
            vec!["yandere", "konachan"]
        );
        assert!(registry.is_active_browse_site("yandere"));
        assert!(registry.is_active_browse_site("konachan"));
        assert!(!registry.is_active_browse_site("pixiv"));
    }

    #[test]
    fn registry_exposes_optional_capabilities_without_fake_methods() {
        let registry = SiteRegistry::default();
        let yandere = registry.site("yandere").unwrap();
        let konachan = registry.site("konachan").unwrap();
        assert!(yandere.remote_favorites().is_some());
        assert!(yandere.collection_download().is_some());
        assert!(konachan.post_lookup().is_some());
        assert!(konachan.remote_favorites().is_none());
        assert!(konachan.collection_download().is_none());
    }

    #[test]
    fn registry_applies_toml_site_enablement() {
        let registry = SiteRegistry::from_enabled([dreamland_site_yandere::SITE_ID]);
        registry.validate().unwrap();
        assert_eq!(registry.descriptors().len(), 1);
        assert!(registry.is_active_browse_site("yandere"));
        assert!(!registry.is_active_browse_site("konachan"));
    }

    #[test]
    fn unsupported_capability_keeps_canonical_error_code() {
        let error = super::unsupported("konachan", "favorites");
        assert_eq!(
            error
                .downcast_ref::<SiteError>()
                .expect("registry should retain SiteError")
                .code,
            SiteErrorCode::UnsupportedCapability
        );
    }
}
