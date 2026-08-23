use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Mutex;

use dreamland_core::{
    BrowserCookie, ContentPolicy, MediaVariant, NetworkPolicy, PoolPage, Post, PostQueryRequest,
    QuerySessionId, SavedQuery, SiteId, SitePage, SiteSession, TagSuggestion, TagSuggestionRequest,
};
use dreamland_runtime::{
    AppConfig, ArchiveRecord, ArchiveRequest, DownloadRecord, DownloadRequest,
};
use dreamland_sites::SiteDescriptor;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

/// Posts from the most recent typed query, keyed by site and post id.
/// Downloads resolve their URL from here rather than trusting a client-supplied
/// one -- the frontend can only ever download a post this backend fetched from
/// the selected site.
struct RuntimeState {
    posts: Mutex<HashMap<String, Post>>,
    auth_session: Mutex<Option<SiteSession>>,
    auth_username: Mutex<Option<String>>,
    sessions: dreamland_runtime::QuerySessionStore,
    downloads: dreamland_runtime::DownloadManager,
    registry: dreamland_sites::SiteRegistry,
}

fn post_cache_key(site_id: &str, post_id: &str) -> String {
    format!("{site_id}:{post_id}")
}

fn resolve_cached_post(
    posts: &Mutex<HashMap<String, Post>>,
    site_id: &str,
    post_id: &str,
    fallback: Option<Post>,
) -> Result<Post, String> {
    let key = post_cache_key(site_id, post_id);
    let mut cache = posts.lock().expect("post cache lock poisoned");
    if let Some(post) = cache.get(&key).cloned() {
        return Ok(post);
    }
    let post =
        fallback.ok_or_else(|| "post not found; reload images before downloading".to_string())?;
    cache.insert(key, post.clone());
    Ok(post)
}

impl RuntimeState {
    fn new(config: &AppConfig) -> anyhow::Result<Self> {
        let registry = dreamland_sites::SiteRegistry::from_enabled(config.enabled_site_ids());
        registry.validate()?;
        Ok(Self {
            posts: Mutex::new(HashMap::new()),
            auth_session: Mutex::new(None),
            auth_username: Mutex::new(None),
            sessions: dreamland_runtime::QuerySessionStore::default(),
            downloads: dreamland_runtime::DownloadManager::open(
                dreamland_runtime::default_state_path(),
                dreamland_runtime::default_cache_path(),
                config.network.clone(),
            )?,
            registry,
        })
    }
}

#[derive(Debug, Deserialize)]
struct QueryInput {
    site: SiteId,
    request: PostQueryRequest,
}

#[derive(Debug, Deserialize)]
struct TagSuggestionInput {
    site: SiteId,
    request: TagSuggestionRequest,
}

#[derive(Debug, Serialize)]
struct AuthStatus {
    authenticated: bool,
    username: Option<String>,
}

#[derive(Debug, Serialize)]
struct AppConfigView {
    download_path: String,
    images_per_page: usize,
    content_policy: ContentPolicy,
    download_variant: MediaVariant,
    network: NetworkPolicy,
}

impl From<&AppConfig> for AppConfigView {
    fn from(config: &AppConfig) -> Self {
        Self {
            download_path: config.download_path.to_string_lossy().into_owned(),
            images_per_page: config.images_per_page,
            content_policy: config.content_policy,
            download_variant: config.download_variant,
            network: config.network.clone(),
        }
    }
}

#[tauri::command]
fn list_sites(state: State<'_, RuntimeState>) -> Vec<SiteDescriptor> {
    state.registry.all_descriptors()
}

#[tauri::command]
fn load_config() -> Result<AppConfigView, String> {
    let config = AppConfig::load_or_default().map_err(|error| error.to_string())?;
    Ok(AppConfigView::from(&config))
}

#[tauri::command]
fn save_config(
    state: State<'_, RuntimeState>,
    download_path: String,
    content_policy: ContentPolicy,
    download_variant: MediaVariant,
    network: NetworkPolicy,
) -> Result<AppConfigView, String> {
    let current = AppConfig::load_or_default().map_err(|error| error.to_string())?;
    let mut config =
        AppConfig::from_user_input(download_path).map_err(|error| error.to_string())?;
    config.images_per_page = current.images_per_page;
    config.sites = current.sites;
    config.content_policy = content_policy;
    config.download_variant = download_variant;
    config.network = network.clone();
    config.save().map_err(|error| error.to_string())?;
    state.downloads.set_network_policy(network);
    Ok(AppConfigView::from(&config))
}

#[tauri::command]
fn detect_proxy() -> dreamland_runtime::ProxyDetection {
    dreamland_runtime::detect_proxy()
}

#[tauri::command]
async fn clear_cache(state: State<'_, RuntimeState>) -> Result<(), String> {
    state
        .downloads
        .clear_cache()
        .await
        .map_err(|error| error.to_string())
}

fn yande_auth_window(
    app: &AppHandle,
    state: &RuntimeState,
    visible: bool,
) -> Result<WebviewWindow, String> {
    if let Some(window) = app.get_webview_window("yande-auth") {
        if visible {
            window.show().map_err(|error| error.to_string())?;
            window.set_focus().map_err(|error| error.to_string())?;
        }
        return Ok(window);
    }
    WebviewWindowBuilder::new(
        app,
        "yande-auth",
        WebviewUrl::External(
            state
                .registry
                .auth_login_url(dreamland_sites::DEFAULT_SITE_ID)
                .map_err(|error| error.to_string())?
                .parse()
                .map_err(|error| format!("invalid login URL: {error}"))?,
        ),
    )
    .title("Sign in to yandere")
    .inner_size(480.0, 760.0)
    .visible(visible)
    .build()
    .map_err(|error| error.to_string())
}

#[tauri::command]
fn begin_auth(app: AppHandle, state: State<'_, RuntimeState>) -> Result<(), String> {
    yande_auth_window(&app, &state, true).map(|_| ())
}

#[tauri::command]
fn open_site(
    app: AppHandle,
    state: State<'_, RuntimeState>,
    site_id: String,
) -> Result<(), String> {
    if !state.registry.is_active_browse_site(&site_id) {
        return Err(format!(
            "'{site_id}' is not a registered, browse-capable site"
        ));
    }
    let url = state
        .registry
        .browser_url(&site_id)
        .map_err(|error| error.to_string())?;
    let label = format!("site-{site_id}");
    if let Some(window) = app.get_webview_window(&label) {
        window.show().map_err(|error| error.to_string())?;
        window.set_focus().map_err(|error| error.to_string())?;
        return Ok(());
    }
    WebviewWindowBuilder::new(
        &app,
        &label,
        WebviewUrl::External(
            url.parse()
                .map_err(|error| format!("invalid site URL: {error}"))?,
        ),
    )
    .title(format!("Dreamland · {site_id}"))
    .inner_size(1100.0, 800.0)
    .build()
    .map(|_| ())
    .map_err(|error| error.to_string())
}

#[tauri::command]
fn open_post(
    app: AppHandle,
    state: State<'_, RuntimeState>,
    site_id: String,
    post_id: String,
) -> Result<(), String> {
    if !state.registry.is_active_browse_site(&site_id) {
        return Err(format!(
            "'{site_id}' is not a registered, browse-capable site"
        ));
    }
    let url = state
        .registry
        .browser_post_url(&site_id, &post_id)
        .map_err(|error| error.to_string())?;
    let label = format!("site-{site_id}-post-{post_id}");
    if let Some(window) = app.get_webview_window(&label) {
        window.show().map_err(|error| error.to_string())?;
        window.set_focus().map_err(|error| error.to_string())?;
        return Ok(());
    }
    WebviewWindowBuilder::new(
        &app,
        &label,
        WebviewUrl::External(
            url.parse()
                .map_err(|error| format!("invalid post URL: {error}"))?,
        ),
    )
    .title(format!("Dreamland · {site_id} post #{post_id}"))
    .inner_size(1100.0, 800.0)
    .build()
    .map(|_| ())
    .map_err(|error| error.to_string())
}

#[tauri::command]
fn open_similar_search(
    app: AppHandle,
    state: State<'_, RuntimeState>,
    site_id: String,
) -> Result<(), String> {
    let site = state
        .registry
        .site(&site_id)
        .map_err(|error| error.to_string())?;
    if !site.descriptor().capabilities.similar_search {
        return Err(format!("site '{site_id}' does not support similar search"));
    }
    let url = state
        .registry
        .browser_similar_url(&site_id)
        .map_err(|error| error.to_string())?;
    let label = format!("site-{site_id}-similar");
    if let Some(window) = app.get_webview_window(&label) {
        window.show().map_err(|error| error.to_string())?;
        window.set_focus().map_err(|error| error.to_string())?;
        return Ok(());
    }
    WebviewWindowBuilder::new(
        &app,
        &label,
        WebviewUrl::External(
            url.parse()
                .map_err(|error| format!("invalid similar-search URL: {error}"))?,
        ),
    )
    .title(format!("Dreamland · {site_id} similar search"))
    .inner_size(1100.0, 800.0)
    .build()
    .map(|_| ())
    .map_err(|error| error.to_string())
}

#[tauri::command]
async fn auth_status(app: AppHandle, state: State<'_, RuntimeState>) -> Result<AuthStatus, String> {
    let window = yande_auth_window(&app, &state, false)?;
    let cookies = window
        .cookies_for_url(
            "https://yande.re/"
                .parse()
                .map_err(|error| format!("invalid yandere URL: {error}"))?,
        )
        .map_err(|error| error.to_string())?;
    let browser_cookies = cookies
        .iter()
        .map(|cookie| BrowserCookie {
            name: cookie.name().to_owned(),
            value: cookie.value().to_owned(),
        })
        .collect::<Vec<_>>();
    let session = state
        .registry
        .auth_session(dreamland_sites::DEFAULT_SITE_ID, &browser_cookies)
        .map_err(|error| error.to_string())?;
    let authenticated = session.is_some();
    *state
        .auth_session
        .lock()
        .expect("auth session lock poisoned") = session.clone();
    let session_for_lookup = state
        .auth_session
        .lock()
        .expect("auth session lock poisoned")
        .clone();
    let username = if authenticated {
        if let Some(session) = session_for_lookup {
            let config = AppConfig::load_or_default().map_err(|error| error.to_string())?;
            state
                .registry
                .current_user(dreamland_sites::DEFAULT_SITE_ID, &session, &config.network)
                .await
                .ok()
        } else {
            None
        }
    } else {
        None
    };
    *state
        .auth_username
        .lock()
        .expect("auth username lock poisoned") = username.clone();
    Ok(AuthStatus {
        authenticated,
        username,
    })
}

#[tauri::command]
fn sign_out(app: AppHandle, state: State<'_, RuntimeState>) -> Result<(), String> {
    *state
        .auth_session
        .lock()
        .expect("auth session lock poisoned") = None;
    *state
        .auth_username
        .lock()
        .expect("auth username lock poisoned") = None;
    if let Some(window) = app.get_webview_window("yande-auth") {
        window.close().map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[tauri::command]
async fn list_pools(
    state: State<'_, RuntimeState>,
    site_id: String,
    query: String,
    page: u32,
    page_size: u16,
) -> Result<PoolPage, String> {
    let config = AppConfig::load_or_default().map_err(|error| error.to_string())?;
    state
        .registry
        .list_pools(&site_id, &query, page, page_size, &config.network)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn query_pool_posts(
    state: State<'_, RuntimeState>,
    site_id: String,
    pool_id: String,
    page: u32,
    page_size: u16,
) -> Result<SitePage, String> {
    let config = AppConfig::load_or_default().map_err(|error| error.to_string())?;
    let page = state
        .registry
        .query_pool_posts(
            &site_id,
            &pool_id,
            config.content_policy,
            page,
            page_size,
            &config.network,
        )
        .await
        .map_err(|error| error.to_string())?;
    let mut posts = state.posts.lock().expect("post cache lock poisoned");
    posts.extend(page.posts.iter().map(|post| {
        (
            post_cache_key(post.post.site.as_str(), &post.post.id),
            post.clone(),
        )
    }));
    Ok(page)
}

#[tauri::command]
async fn enqueue_pool_zip(
    state: State<'_, RuntimeState>,
    pool_id: String,
    pool_name: String,
) -> Result<ArchiveRecord, String> {
    let session = state
        .auth_session
        .lock()
        .expect("auth session lock poisoned")
        .clone()
        .ok_or_else(|| "yandere login is required to download a pool ZIP".to_owned())?;
    let config = AppConfig::load_or_default().map_err(|error| error.to_string())?;
    let url = state
        .registry
        .pool_zip_url(dreamland_sites::DEFAULT_SITE_ID, &pool_id)
        .map_err(|error| error.to_string())?;
    state
        .downloads
        .enqueue_archive(ArchiveRequest {
            site: SiteId::new(dreamland_sites::DEFAULT_SITE_ID),
            pool_id,
            pool_name,
            source_url: url,
            download_root: config.download_path,
            cookie_header: session.cookie_header().to_owned(),
        })
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn list_favorites(
    state: State<'_, RuntimeState>,
    page: u32,
    page_size: u16,
) -> Result<SitePage, String> {
    let session = state
        .auth_session
        .lock()
        .expect("auth session lock poisoned")
        .clone()
        .ok_or_else(|| "yandere login is required to view favorites".to_owned())?;
    let config = AppConfig::load_or_default().map_err(|error| error.to_string())?;
    let page = state
        .registry
        .list_favorites(
            dreamland_sites::DEFAULT_SITE_ID,
            &session,
            config.content_policy,
            page,
            page_size,
            &config.network,
        )
        .await
        .map_err(|error| error.to_string())?;
    let mut posts = state.posts.lock().expect("post cache lock poisoned");
    posts.extend(page.posts.iter().map(|post| {
        (
            post_cache_key(post.post.site.as_str(), &post.post.id),
            post.clone(),
        )
    }));
    Ok(page)
}

#[tauri::command]
async fn list_saved_queries(
    state: State<'_, RuntimeState>,
    site_id: String,
) -> Result<Vec<SavedQuery>, String> {
    if !state.registry.is_active_browse_site(&site_id) {
        return Err(format!(
            "'{site_id}' is not a registered, browse-capable site"
        ));
    }
    let site = SiteId::new(site_id);
    state
        .downloads
        .saved_queries(&site)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn save_saved_query(
    state: State<'_, RuntimeState>,
    saved: SavedQuery,
) -> Result<SavedQuery, String> {
    if !state.registry.is_active_browse_site(saved.site.as_str()) {
        return Err(format!(
            "'{}' is not a registered, browse-capable site",
            saved.site.as_str()
        ));
    }
    let site = saved.site.clone();
    state
        .downloads
        .save_saved_query(&site, saved)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn delete_saved_query(
    state: State<'_, RuntimeState>,
    site_id: String,
    id: String,
) -> Result<(), String> {
    if !state.registry.is_active_browse_site(&site_id) {
        return Err(format!(
            "'{site_id}' is not a registered, browse-capable site"
        ));
    }
    let site = SiteId::new(site_id);
    state
        .downloads
        .delete_saved_query(&site, &id)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn move_saved_query(
    state: State<'_, RuntimeState>,
    site_id: String,
    id: String,
    direction: i8,
) -> Result<(), String> {
    if !state.registry.is_active_browse_site(&site_id) {
        return Err(format!(
            "'{site_id}' is not a registered, browse-capable site"
        ));
    }
    let site = SiteId::new(site_id);
    state
        .downloads
        .move_saved_query(&site, &id, direction)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn set_favorite(
    state: State<'_, RuntimeState>,
    post_id: String,
    favorite: bool,
) -> Result<(), String> {
    let session = state
        .auth_session
        .lock()
        .expect("auth session lock poisoned")
        .clone()
        .ok_or_else(|| "yandere login is required to change favorites".to_owned())?;
    let config = AppConfig::load_or_default().map_err(|error| error.to_string())?;
    state
        .registry
        .set_favorite(
            dreamland_sites::DEFAULT_SITE_ID,
            &post_id,
            favorite,
            &session,
            &config.network,
        )
        .await
        .map_err(|error| error.to_string())
}

async fn execute_query(
    state: &RuntimeState,
    site_id: &str,
    request: PostQueryRequest,
) -> Result<SitePage, String> {
    if !state.registry.is_active_browse_site(site_id) {
        return Err(format!(
            "'{site_id}' is not a registered, browse-capable site"
        ));
    }
    let session = state.sessions.start(SiteId::new(site_id), request.clone());
    let operation = state
        .sessions
        .begin_operation(&session.id)
        .ok_or_else(|| "query session was cancelled before dispatch".to_string())?;
    execute_session_query(state, session, operation, true).await
}

async fn execute_session_query(
    state: &RuntimeState,
    session: dreamland_runtime::QuerySession,
    operation: dreamland_runtime::SessionOperation,
    replace_cache: bool,
) -> Result<SitePage, String> {
    let config = AppConfig::load_or_default().map_err(|error| error.to_string())?;
    let result = state
        .registry
        .query_posts(session.site.as_str(), &session.request, &config.network)
        .await;
    let mut page = match result {
        Ok(page) => page,
        Err(error) => {
            state.sessions.cancel(&session.id);
            return Err(error.to_string());
        }
    };
    if !state.sessions.is_current(&operation) {
        return Err("stale query result discarded".to_string());
    }
    page.session = Some(session.id);
    let mut posts = state.posts.lock().expect("post cache lock poisoned");
    if replace_cache {
        posts.clear();
    }
    posts.extend(page.posts.iter().map(|post| {
        (
            post_cache_key(post.post.site.as_str(), &post.post.id),
            post.clone(),
        )
    }));
    Ok(page)
}

#[tauri::command]
async fn query_posts(
    state: State<'_, RuntimeState>,
    input: QueryInput,
) -> Result<SitePage, String> {
    execute_query(&state, input.site.as_str(), input.request).await
}

#[tauri::command]
async fn lookup_post(
    state: State<'_, RuntimeState>,
    site_id: String,
    post_id: String,
    content_policy: ContentPolicy,
) -> Result<Post, String> {
    let config = AppConfig::load_or_default().map_err(|error| error.to_string())?;
    let post = state
        .registry
        .lookup_post(&site_id, &post_id, content_policy, &config.network)
        .await
        .map_err(|error| error.to_string())?;
    state
        .posts
        .lock()
        .expect("post cache lock poisoned")
        .insert(post_cache_key(&site_id, &post_id), post.clone());
    Ok(post)
}

#[tauri::command]
async fn load_detail_image(
    state: State<'_, RuntimeState>,
    site_id: String,
    post_id: String,
    refresh: Option<bool>,
) -> Result<String, String> {
    if !state.registry.is_active_browse_site(&site_id) {
        let error = format!("'{site_id}' is not a registered, browse-capable site");
        dreamland_runtime::log_detail_failure(&site_id, &post_id, &error);
        return Err(error);
    }
    let cache_root = dreamland_runtime::default_detail_cache_path();
    let config = AppConfig::load_or_default().map_err(|error| {
        let error = error.to_string();
        dreamland_runtime::log_detail_failure(&site_id, &post_id, &error);
        error
    })?;
    if refresh.unwrap_or(false) {
        let source_url = detail_source_url(&state, &site_id, &post_id)?;
        return dreamland_runtime::refresh_detail_image_at(
            &source_url,
            &site_id,
            &post_id,
            &cache_root,
            &config.network,
        )
        .await
        .map(|path| path.to_string_lossy().into_owned())
        .map_err(|error| {
            dreamland_runtime::log_detail_failure(&site_id, &post_id, &error.to_string());
            error.to_string()
        });
    }
    if let Some(path) =
        dreamland_runtime::find_existing_image_at(&config.download_path, &site_id, &post_id)
            .await
            .map_err(|error| error.to_string())?
    {
        return Ok(path.to_string_lossy().into_owned());
    }
    if let Some(path) =
        dreamland_runtime::find_cached_detail_image_at(&site_id, &post_id, &cache_root)
            .await
            .map_err(|error| error.to_string())?
    {
        return Ok(path.to_string_lossy().into_owned());
    }
    let source_url = detail_source_url(&state, &site_id, &post_id)?;
    dreamland_runtime::cache_detail_image_at(
        &source_url,
        &site_id,
        &post_id,
        &cache_root,
        &config.network,
    )
    .await
    .map(|path| path.to_string_lossy().into_owned())
    .map_err(|error| {
        dreamland_runtime::log_detail_failure(&site_id, &post_id, &error.to_string());
        error.to_string()
    })
}

fn detail_source_url(state: &RuntimeState, site_id: &str, post_id: &str) -> Result<String, String> {
    let posts = state.posts.lock().expect("post cache lock poisoned");
    match posts
        .get(&post_cache_key(site_id, post_id))
        .and_then(|post| post.full_url.clone())
    {
        Some(url) => Ok(url),
        None => {
            let error = "full image URL is unavailable for this post".to_owned();
            dreamland_runtime::log_detail_failure(site_id, post_id, &error);
            Err(error)
        }
    }
}

#[tauri::command]
async fn related_tags(
    state: State<'_, RuntimeState>,
    site_id: String,
    tags: Vec<String>,
    limit: u16,
) -> Result<Vec<dreamland_core::RelatedTag>, String> {
    let config = AppConfig::load_or_default().map_err(|error| error.to_string())?;
    state
        .registry
        .related_tags(
            &site_id,
            &dreamland_core::RelatedTagRequest { tags, limit },
            &config.network,
        )
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn continue_query(
    state: State<'_, RuntimeState>,
    session: QuerySessionId,
) -> Result<SitePage, String> {
    let (session_state, operation) = state
        .sessions
        .begin_next_page(&session)
        .ok_or_else(|| "query session has no next page".to_string())?;
    execute_session_query(&state, session_state, operation, false).await
}

#[tauri::command]
fn cancel_query(state: State<'_, RuntimeState>, session: QuerySessionId) -> Result<(), String> {
    if state.sessions.cancel(&session) {
        Ok(())
    } else {
        Err("query session not found or already cancelled".to_string())
    }
}

#[tauri::command]
async fn suggest_tags(
    state: State<'_, RuntimeState>,
    input: TagSuggestionInput,
) -> Result<Vec<TagSuggestion>, String> {
    if !state.registry.is_active_browse_site(input.site.as_str()) {
        return Err(format!(
            "'{}' is not a registered, browse-capable site",
            input.site.as_str()
        ));
    }
    let config = AppConfig::load_or_default().map_err(|error| error.to_string())?;
    state
        .registry
        .suggest_tags(input.site.as_str(), &input.request, &config.network)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn enqueue_download(
    state: State<'_, RuntimeState>,
    site_id: String,
    post_id: String,
    variant: MediaVariant,
) -> Result<DownloadRecord, String> {
    if !state.registry.is_active_browse_site(&site_id) {
        return Err(format!(
            "'{site_id}' is not a registered, browse-capable site"
        ));
    }
    let config = AppConfig::load_or_default().map_err(|error| error.to_string())?;
    let cached_post = {
        let cache = state.posts.lock().expect("post cache lock poisoned");
        cache.get(&post_cache_key(&site_id, &post_id)).cloned()
    };
    let fallback_post = if cached_post.is_none() {
        Some(
            state
                .registry
                .lookup_post(&site_id, &post_id, config.content_policy, &config.network)
                .await
                .map_err(|error| error.to_string())?,
        )
    } else {
        None
    };
    let post = resolve_cached_post(
        &state.posts,
        &site_id,
        &post_id,
        fallback_post.or(cached_post),
    )?;
    let url = state
        .registry
        .resolve_media_url(&site_id, &post, variant)
        .ok_or_else(|| {
            format!(
                "requested {variant:?} variant is unavailable for post {}",
                post.post.id
            )
        })?;
    state
        .downloads
        .enqueue(DownloadRequest {
            site: SiteId::new(site_id),
            post_id: post.post.id.clone(),
            variant,
            source_url: url.to_owned(),
            download_root: config.download_path,
            metadata: post,
        })
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn cancel_download(state: State<'_, RuntimeState>, id: String) -> Result<(), String> {
    state
        .downloads
        .cancel(&id)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn retry_download(
    state: State<'_, RuntimeState>,
    id: String,
) -> Result<DownloadRecord, String> {
    state
        .downloads
        .retry(&id)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn list_downloads(
    state: State<'_, RuntimeState>,
    limit: u32,
) -> Result<Vec<DownloadRecord>, String> {
    state
        .downloads
        .records(limit.min(100))
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn list_download_history(
    state: State<'_, RuntimeState>,
    limit: u32,
    offset: u32,
) -> Result<Vec<DownloadRecord>, String> {
    state
        .downloads
        .history_page(limit.min(100), offset)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn list_archives(
    state: State<'_, RuntimeState>,
    limit: u32,
) -> Result<Vec<ArchiveRecord>, String> {
    state
        .downloads
        .archives(limit.min(100))
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn cancel_archive(state: State<'_, RuntimeState>, id: String) -> Result<(), String> {
    state
        .downloads
        .cancel_archive(&id)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn open_download(path: String) -> Result<(), String> {
    let path = PathBuf::from(path);
    if !path.is_absolute() {
        return Err("download path must be absolute".to_owned());
    }
    if !path.is_file() {
        return Err("downloaded file no longer exists".to_owned());
    }

    #[cfg(target_os = "macos")]
    let mut command = Command::new("open");
    #[cfg(target_os = "windows")]
    let mut command = {
        let mut command = Command::new("cmd");
        command.args(["/C", "start", ""]);
        command
    };
    #[cfg(all(unix, not(target_os = "macos")))]
    let mut command = Command::new("xdg-open");

    command
        .arg(path)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("could not open downloaded file: {error}"))
}

pub fn run() {
    let config =
        AppConfig::load_or_default().expect("Dreamland runtime configuration must be loadable");
    let runtime_state =
        RuntimeState::new(&config).expect("Dreamland local SQLite state must be initializable");
    tauri::Builder::default()
        .manage(runtime_state)
        .setup(|app| {
            let downloads = app.state::<RuntimeState>().downloads.clone();
            downloads.set_detail_cache_root(dreamland_runtime::default_detail_cache_path());
            tauri::async_runtime::spawn(downloads.worker());
            tauri::async_runtime::spawn(downloads.archive_worker());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_sites,
            load_config,
            save_config,
            detect_proxy,
            clear_cache,
            begin_auth,
            open_site,
            open_post,
            open_similar_search,
            auth_status,
            sign_out,
            list_pools,
            query_pool_posts,
            enqueue_pool_zip,
            list_favorites,
            list_saved_queries,
            save_saved_query,
            delete_saved_query,
            move_saved_query,
            set_favorite,
            query_posts,
            lookup_post,
            load_detail_image,
            related_tags,
            continue_query,
            cancel_query,
            suggest_tags,
            enqueue_download,
            cancel_download,
            retry_download,
            list_downloads,
            list_download_history,
            list_archives,
            cancel_archive,
            open_download
        ])
        .run(tauri::generate_context!())
        .expect("error while running Dreamland");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_post(site: &str, id: &str) -> Post {
        Post {
            post: dreamland_core::PostRef {
                site: SiteId::new(site),
                id: id.to_owned(),
            },
            tags: vec!["dreamland".to_owned()],
            author: None,
            creator_id: None,
            md5: None,
            source: None,
            parent_id: None,
            has_children: false,
            created_at: None,
            width: Some(100),
            height: Some(100),
            rating: dreamland_core::Rating::Safe,
            score: None,
            preview_url: None,
            sample_url: None,
            full_url: Some("https://example.test/full.png".to_owned()),
            file_size: None,
        }
    }

    #[test]
    fn download_post_falls_back_after_cached_route_replaces_runtime_cache() {
        let posts = Mutex::new(HashMap::new());
        let post = test_post("yandere", "123");

        let resolved = resolve_cached_post(&posts, "yandere", "123", Some(post.clone()))
            .expect("the authoritative lookup should restore the post cache");

        assert_eq!(resolved, post);
        assert!(posts
            .lock()
            .expect("post cache lock poisoned")
            .contains_key("yandere:123"));
    }
}
