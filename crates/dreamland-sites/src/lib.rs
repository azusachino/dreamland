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

#[cfg(test)]
mod tests {
    use super::descriptors;

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
}
