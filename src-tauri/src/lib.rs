use std::collections::HashMap;
use std::sync::Mutex;

use dreamland_core::{MediaVariant, Post};
use dreamland_runtime::AppConfig;
use dreamland_sites::SiteDescriptor;
use tauri::State;

/// Posts from the most recent `load_images` call for the active site, keyed
/// by post id. `download_image` resolves its URL from here rather than
/// trusting a client-supplied one -- the frontend can only ever download a
/// post this backend already fetched from the real site.
#[derive(Default)]
struct PostCache(Mutex<HashMap<String, Post>>);

#[tauri::command]
fn list_sites() -> Vec<SiteDescriptor> {
    dreamland_sites::descriptors()
}

#[tauri::command]
fn load_config() -> Result<AppConfig, String> {
    AppConfig::load_or_default(&dreamland_site_yandere::default_config().api_url)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn save_config(download_path: String, api_url: String) -> Result<AppConfig, String> {
    let config =
        AppConfig::from_user_input(download_path, api_url).map_err(|error| error.to_string())?;
    config.save().map_err(|error| error.to_string())?;
    Ok(config)
}

#[tauri::command]
async fn load_images(
    state: State<'_, PostCache>,
    site_id: String,
    page: usize,
) -> Result<Vec<Post>, String> {
    if !dreamland_sites::is_active_browse_site(&site_id) {
        return Err(format!(
            "'{site_id}' is not a registered, browse-capable site"
        ));
    }
    let config = AppConfig::load_or_default(&dreamland_site_yandere::default_config().api_url)
        .map_err(|error| error.to_string())?;
    let posts = dreamland_site_yandere::fetch_images(&config.api_url, page)
        .await
        .map_err(|error| error.to_string())?;

    let mut cache = state.0.lock().expect("post cache lock poisoned");
    cache.clear();
    cache.extend(
        posts
            .iter()
            .map(|post| (post.post.id.clone(), post.clone())),
    );

    Ok(posts)
}

#[tauri::command]
async fn download_image(
    state: State<'_, PostCache>,
    site_id: String,
    post_id: String,
    variant: MediaVariant,
) -> Result<String, String> {
    if !dreamland_sites::is_active_browse_site(&site_id) {
        return Err(format!(
            "'{site_id}' is not a registered, browse-capable site"
        ));
    }
    let post = {
        let cache = state.0.lock().expect("post cache lock poisoned");
        cache
            .get(&post_id)
            .cloned()
            .ok_or_else(|| "post not found; reload images before downloading".to_string())?
    };
    let config = AppConfig::load_or_default(&dreamland_site_yandere::default_config().api_url)
        .map_err(|error| error.to_string())?;
    let url = dreamland_site_yandere::variant_url(&post, variant);
    let final_path = dreamland_runtime::download_image(url, &post.post.id, &config.download_path)
        .await
        .map_err(|error| error.to_string())?;
    Ok(final_path.display().to_string())
}

pub fn run() {
    tauri::Builder::default()
        .manage(PostCache::default())
        .invoke_handler(tauri::generate_handler![
            list_sites,
            load_config,
            save_config,
            load_images,
            download_image
        ])
        .run(tauri::generate_context!())
        .expect("error while running Dreamland");
}
