use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Mutex;

use dreamland_core::{
    ContentPolicy, MediaVariant, NetworkPolicy, PoolPage, Post, PostQueryRequest, QuerySessionId,
    SavedQuery, SiteId, SitePage, TagSuggestion, TagSuggestionRequest,
};
use dreamland_runtime::{AppConfig, DownloadRecord, DownloadRequest};
use dreamland_sites::SiteDescriptor;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State, WebviewUrl, WebviewWindowBuilder};

/// Posts from the most recent typed query, keyed by post id. Downloads resolve
/// their URL from here rather than trusting a client-supplied one -- the
/// frontend can only ever download a post this backend fetched from the site.
struct RuntimeState {
    posts: Mutex<HashMap<String, Post>>,
    auth_cookie: Mutex<Option<String>>,
    auth_username: Mutex<Option<String>>,
    sessions: dreamland_runtime::QuerySessionStore,
    downloads: dreamland_runtime::DownloadManager,
}

impl RuntimeState {
    fn new(config: &AppConfig) -> anyhow::Result<Self> {
        Ok(Self {
            posts: Mutex::new(HashMap::new()),
            auth_cookie: Mutex::new(None),
            auth_username: Mutex::new(None),
            sessions: dreamland_runtime::QuerySessionStore::default(),
            downloads: dreamland_runtime::DownloadManager::open(
                dreamland_runtime::default_state_path(),
                dreamland_runtime::default_cache_path(),
                config.network.clone(),
            )?,
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
    network: NetworkPolicy,
}

impl From<&AppConfig> for AppConfigView {
    fn from(config: &AppConfig) -> Self {
        Self {
            download_path: config.download_path.to_string_lossy().into_owned(),
            images_per_page: config.images_per_page,
            content_policy: config.content_policy,
            network: config.network.clone(),
        }
    }
}

#[tauri::command]
fn list_sites() -> Vec<SiteDescriptor> {
    dreamland_sites::descriptors()
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
    network: NetworkPolicy,
) -> Result<AppConfigView, String> {
    let current = AppConfig::load_or_default().map_err(|error| error.to_string())?;
    let mut config =
        AppConfig::from_user_input(download_path).map_err(|error| error.to_string())?;
    config.images_per_page = current.images_per_page;
    config.content_policy = content_policy;
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
fn begin_auth(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("yande-auth") {
        window.show().map_err(|error| error.to_string())?;
        window.set_focus().map_err(|error| error.to_string())?;
        return Ok(());
    }
    WebviewWindowBuilder::new(
        &app,
        "yande-auth",
        WebviewUrl::External(
            "https://yande.re/user/login"
                .parse()
                .map_err(|error| format!("invalid login URL: {error}"))?,
        ),
    )
    .title("Sign in to Yande.re")
    .inner_size(480.0, 760.0)
    .build()
    .map(|_| ())
    .map_err(|error| error.to_string())
}

#[tauri::command]
async fn auth_status(app: AppHandle, state: State<'_, RuntimeState>) -> Result<AuthStatus, String> {
    let Some(window) = app.get_webview_window("yande-auth") else {
        return Ok(AuthStatus {
            authenticated: false,
            username: None,
        });
    };
    let cookies = window
        .cookies_for_url(
            "https://yande.re/"
                .parse()
                .map_err(|error| format!("invalid Yande URL: {error}"))?,
        )
        .map_err(|error| error.to_string())?;
    let authenticated = cookies.iter().any(|cookie| cookie.name() == "user_id");
    let user_id = cookies
        .iter()
        .find(|cookie| cookie.name() == "user_id")
        .map(|cookie| cookie.value().to_owned());
    let cookie_header = cookies
        .iter()
        .map(|cookie| format!("{}={}", cookie.name(), cookie.value()))
        .collect::<Vec<_>>()
        .join("; ");
    *state.auth_cookie.lock().expect("auth cookie lock poisoned") =
        authenticated.then_some(cookie_header);
    let cookie_for_lookup = state
        .auth_cookie
        .lock()
        .expect("auth cookie lock poisoned")
        .clone();
    let username = if authenticated {
        if let (Some(user_id), Some(cookie_header)) = (user_id, cookie_for_lookup) {
            let config = AppConfig::load_or_default().map_err(|error| error.to_string())?;
            dreamland_sites::current_user(
                dreamland_sites::DEFAULT_SITE_ID,
                &user_id,
                &cookie_header,
                &config.network,
            )
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
    *state.auth_cookie.lock().expect("auth cookie lock poisoned") = None;
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
    _state: State<'_, RuntimeState>,
    page: u32,
    page_size: u16,
) -> Result<PoolPage, String> {
    let config = AppConfig::load_or_default().map_err(|error| error.to_string())?;
    dreamland_sites::list_pools(
        dreamland_sites::DEFAULT_SITE_ID,
        page,
        page_size,
        &config.network,
    )
    .await
    .map_err(|error| error.to_string())
}

#[tauri::command]
async fn query_pool_posts(
    state: State<'_, RuntimeState>,
    pool_id: String,
    page: u32,
    page_size: u16,
) -> Result<SitePage, String> {
    let config = AppConfig::load_or_default().map_err(|error| error.to_string())?;
    let page = dreamland_sites::query_pool_posts(
        dreamland_sites::DEFAULT_SITE_ID,
        &pool_id,
        config.content_policy,
        page,
        page_size,
        &config.network,
    )
    .await
    .map_err(|error| error.to_string())?;
    let mut posts = state.posts.lock().expect("post cache lock poisoned");
    posts.extend(
        page.posts
            .iter()
            .map(|post| (post.post.id.clone(), post.clone())),
    );
    Ok(page)
}

#[tauri::command]
async fn list_favorites(
    state: State<'_, RuntimeState>,
    page: u32,
    page_size: u16,
) -> Result<SitePage, String> {
    let cookie = state
        .auth_cookie
        .lock()
        .expect("auth cookie lock poisoned")
        .clone()
        .ok_or_else(|| "Yande login is required to view favorites".to_owned())?;
    let username = state
        .auth_username
        .lock()
        .expect("auth username lock poisoned")
        .clone()
        .ok_or_else(|| "Refresh Yande login status before viewing favorites".to_owned())?;
    let config = AppConfig::load_or_default().map_err(|error| error.to_string())?;
    let page = dreamland_sites::list_favorites(
        dreamland_sites::DEFAULT_SITE_ID,
        &username,
        &cookie,
        config.content_policy,
        page,
        page_size,
        &config.network,
    )
    .await
    .map_err(|error| error.to_string())?;
    let mut posts = state.posts.lock().expect("post cache lock poisoned");
    posts.extend(
        page.posts
            .iter()
            .map(|post| (post.post.id.clone(), post.clone())),
    );
    Ok(page)
}

#[tauri::command]
async fn list_saved_queries(state: State<'_, RuntimeState>) -> Result<Vec<SavedQuery>, String> {
    let site = SiteId::new(dreamland_sites::DEFAULT_SITE_ID);
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
    let site = SiteId::new(dreamland_sites::DEFAULT_SITE_ID);
    if saved.site != site {
        return Err(format!("site '{}' is not active", saved.site.as_str()));
    }
    state
        .downloads
        .save_saved_query(&site, saved)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn delete_saved_query(state: State<'_, RuntimeState>, id: String) -> Result<(), String> {
    let site = SiteId::new(dreamland_sites::DEFAULT_SITE_ID);
    state
        .downloads
        .delete_saved_query(&site, &id)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn set_favorite(
    state: State<'_, RuntimeState>,
    post_id: String,
    favorite: bool,
) -> Result<(), String> {
    let cookie = state
        .auth_cookie
        .lock()
        .expect("auth cookie lock poisoned")
        .clone()
        .ok_or_else(|| "Yande login is required to change favorites".to_owned())?;
    let config = AppConfig::load_or_default().map_err(|error| error.to_string())?;
    dreamland_sites::set_favorite(
        dreamland_sites::DEFAULT_SITE_ID,
        &post_id,
        favorite,
        &cookie,
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
    if !dreamland_sites::is_active_browse_site(site_id) {
        return Err(format!(
            "'{site_id}' is not a registered, browse-capable site"
        ));
    }
    let session = state.sessions.start(SiteId::new(site_id), request.clone());
    let operation = state
        .sessions
        .begin_operation(&session.id)
        .ok_or_else(|| "query session was cancelled before dispatch".to_string())?;
    execute_session_query(state, session, operation).await
}

async fn execute_session_query(
    state: &RuntimeState,
    session: dreamland_runtime::QuerySession,
    operation: dreamland_runtime::SessionOperation,
) -> Result<SitePage, String> {
    let config = AppConfig::load_or_default().map_err(|error| error.to_string())?;
    let result =
        dreamland_sites::query_posts(session.site.as_str(), &session.request, &config.network)
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
    posts.clear();
    posts.extend(
        page.posts
            .iter()
            .map(|post| (post.post.id.clone(), post.clone())),
    );
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
async fn continue_query(
    state: State<'_, RuntimeState>,
    session: QuerySessionId,
) -> Result<SitePage, String> {
    let (session_state, operation) = state
        .sessions
        .begin_next_page(&session)
        .ok_or_else(|| "query session has no next page".to_string())?;
    execute_session_query(&state, session_state, operation).await
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
async fn suggest_tags(input: TagSuggestionInput) -> Result<Vec<TagSuggestion>, String> {
    if !dreamland_sites::is_active_browse_site(input.site.as_str()) {
        return Err(format!(
            "'{}' is not a registered, browse-capable site",
            input.site.as_str()
        ));
    }
    let config = AppConfig::load_or_default().map_err(|error| error.to_string())?;
    dreamland_sites::suggest_tags(input.site.as_str(), &input.request, &config.network)
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
    if !dreamland_sites::is_active_browse_site(&site_id) {
        return Err(format!(
            "'{site_id}' is not a registered, browse-capable site"
        ));
    }
    let post = {
        let cache = state.posts.lock().expect("post cache lock poisoned");
        cache
            .get(&post_id)
            .cloned()
            .ok_or_else(|| "post not found; reload images before downloading".to_string())?
    };
    let config = AppConfig::load_or_default().map_err(|error| error.to_string())?;
    let url = dreamland_sites::resolve_media_url(&site_id, &post, variant).ok_or_else(|| {
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
            tauri::async_runtime::spawn(downloads.worker());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_sites,
            load_config,
            save_config,
            detect_proxy,
            begin_auth,
            auth_status,
            sign_out,
            list_pools,
            query_pool_posts,
            list_favorites,
            list_saved_queries,
            save_saved_query,
            delete_saved_query,
            set_favorite,
            query_posts,
            continue_query,
            cancel_query,
            suggest_tags,
            enqueue_download,
            cancel_download,
            retry_download,
            list_downloads,
            list_download_history,
            open_download
        ])
        .run(tauri::generate_context!())
        .expect("error while running Dreamland");
}
