pub use dreamland_core::SiteDescriptor;

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
    use super::{descriptors, is_active_browse_site};

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
}
