use std::collections::HashMap;
use std::sync::Mutex;

use dreamland_core::{
    MediaVariant, NetworkPolicy, Post, PostQueryRequest, QuerySessionId, SiteId, SitePage,
    TagSuggestion, TagSuggestionRequest,
};
use dreamland_runtime::AppConfig;
use dreamland_sites::SiteDescriptor;
use serde::Deserialize;
use tauri::State;

/// Posts from the most recent typed query, keyed by post id. Downloads resolve
/// their URL from here rather than trusting a client-supplied one -- the
/// frontend can only ever download a post this backend fetched from the site.
struct RuntimeState {
    posts: Mutex<HashMap<String, Post>>,
    sessions: dreamland_runtime::QuerySessionStore,
}

impl Default for RuntimeState {
    fn default() -> Self {
        Self {
            posts: Mutex::new(HashMap::new()),
            sessions: dreamland_runtime::QuerySessionStore::default(),
        }
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

#[tauri::command]
fn list_sites() -> Vec<SiteDescriptor> {
    dreamland_sites::descriptors()
}

#[tauri::command]
fn load_config() -> Result<AppConfig, String> {
    AppConfig::load_or_default(
        &dreamland_sites::default_api_url(dreamland_sites::DEFAULT_SITE_ID)
            .expect("the active default site must provide a default API URL"),
    )
    .map_err(|error| error.to_string())
}

#[tauri::command]
fn save_config(
    download_path: String,
    api_url: String,
    network: NetworkPolicy,
) -> Result<AppConfig, String> {
    let mut config =
        AppConfig::from_user_input(download_path, api_url).map_err(|error| error.to_string())?;
    config.network = network;
    config.save().map_err(|error| error.to_string())?;
    Ok(config)
}

#[tauri::command]
fn detect_proxy() -> dreamland_runtime::ProxyDetection {
    dreamland_runtime::detect_proxy()
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
    let config = AppConfig::load_or_default(
        &dreamland_sites::default_api_url(dreamland_sites::DEFAULT_SITE_ID)
            .expect("the active default site must provide a default API URL"),
    )
    .map_err(|error| error.to_string())?;
    let result = dreamland_sites::query_posts(
        session.site.as_str(),
        &config.api_url,
        &session.request,
        &config.network,
    )
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
    let config = AppConfig::load_or_default(
        &dreamland_sites::default_api_url(dreamland_sites::DEFAULT_SITE_ID)
            .expect("the active default site must provide a default API URL"),
    )
    .map_err(|error| error.to_string())?;
    dreamland_sites::suggest_tags(
        input.site.as_str(),
        &config.api_url,
        &input.request,
        &config.network,
    )
    .await
    .map_err(|error| error.to_string())
}

#[tauri::command]
async fn download_image(
    state: State<'_, RuntimeState>,
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
        let cache = state.posts.lock().expect("post cache lock poisoned");
        cache
            .get(&post_id)
            .cloned()
            .ok_or_else(|| "post not found; reload images before downloading".to_string())?
    };
    let config = AppConfig::load_or_default(
        &dreamland_sites::default_api_url(dreamland_sites::DEFAULT_SITE_ID)
            .expect("the active default site must provide a default API URL"),
    )
    .map_err(|error| error.to_string())?;
    let url = dreamland_sites::resolve_media_url(&site_id, &post, variant).ok_or_else(|| {
        format!(
            "requested {variant:?} variant is unavailable for post {}",
            post.post.id
        )
    })?;
    let final_path = dreamland_runtime::download_image(
        url,
        &post.post.id,
        &config.download_path,
        &config.network,
    )
    .await
    .map_err(|error| error.to_string())?;
    Ok(final_path.display().to_string())
}

pub fn run() {
    tauri::Builder::default()
        .manage(RuntimeState::default())
        .invoke_handler(tauri::generate_handler![
            list_sites,
            load_config,
            save_config,
            detect_proxy,
            query_posts,
            continue_query,
            cancel_query,
            suggest_tags,
            download_image
        ])
        .run(tauri::generate_context!())
        .expect("error while running Dreamland");
}
