use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

#[cfg(mobile)]
use tauri_plugin_android_fs::{AndroidFsExt, PublicImageDir};
#[cfg(mobile)]
use tauri_plugin_notification::NotificationExt;

use dreamland_core::BrowserCookie;
use dreamland_runtime::AppConfig;

use crate::{AuthStatus, RuntimeState};

/// Platform-divergent behavior, selected once at compile time via
/// `ActivePlatform` below. Each `#[tauri::command]` and `run()` calls
/// through this trait instead of scattering `#[cfg(mobile)]` through
/// business logic; the trait bound keeps both implementations honest about
/// which behaviors they must cover.
pub(crate) trait Platform {
    /// Registers any platform-only Tauri plugins. Defaults to a no-op.
    fn register_plugins<R: tauri::Runtime>(builder: tauri::Builder<R>) -> tauri::Builder<R> {
        builder
    }

    /// Points dreamland-runtime's XDG-style env lookups at Tauri's
    /// mobile-aware app directories. A no-op where they're already correct.
    fn configure_dirs(app: &tauri::App);

    fn begin_auth(app: &AppHandle, state: &RuntimeState) -> Result<(), String>;

    fn auth_status(
        app: &AppHandle,
        state: &RuntimeState,
    ) -> impl std::future::Future<Output = Result<AuthStatus, String>> + Send;

    /// Wires a background task that reacts to completed downloads -- e.g.
    /// publishing them somewhere the user can find them, notifying the user.
    /// A no-op where nothing needs to.
    fn handle_completed_downloads(app: &tauri::App, downloads: &dreamland_runtime::DownloadManager);
}

#[cfg(not(mobile))]
pub(crate) struct Desktop;

#[cfg(not(mobile))]
impl Desktop {
    fn auth_window(
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
}

#[cfg(not(mobile))]
impl Platform for Desktop {
    fn configure_dirs(_app: &tauri::App) {}

    fn begin_auth(app: &AppHandle, state: &RuntimeState) -> Result<(), String> {
        Self::auth_window(app, state, true).map(|_| ())
    }

    async fn auth_status(app: &AppHandle, state: &RuntimeState) -> Result<AuthStatus, String> {
        let window = Self::auth_window(app, state, false)?;
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

    fn handle_completed_downloads(
        _app: &tauri::App,
        _downloads: &dreamland_runtime::DownloadManager,
    ) {
    }
}

#[cfg(mobile)]
pub(crate) struct Mobile;

#[cfg(mobile)]
impl Platform for Mobile {
    fn register_plugins<R: tauri::Runtime>(builder: tauri::Builder<R>) -> tauri::Builder<R> {
        builder.plugin(tauri_plugin_android_fs::init())
    }

    fn configure_dirs(app: &tauri::App) {
        let path = app.path();
        if let Ok(data_dir) = path.app_data_dir() {
            std::env::set_var("XDG_DATA_HOME", &data_dir);
            std::env::set_var("XDG_STATE_HOME", &data_dir);
            std::env::set_var("XDG_CONFIG_HOME", &data_dir);
            std::env::set_var("DREAMLAND_DOWNLOAD_DIR", data_dir.join("images"));
        }
        if let Ok(cache_dir) = path.app_cache_dir() {
            std::env::set_var("XDG_CACHE_HOME", &cache_dir);
        }
    }

    fn begin_auth(_app: &AppHandle, _state: &RuntimeState) -> Result<(), String> {
        Err("sign-in is not available on Android yet".to_owned())
    }

    async fn auth_status(_app: &AppHandle, _state: &RuntimeState) -> Result<AuthStatus, String> {
        Ok(AuthStatus {
            authenticated: false,
            username: None,
        })
    }

    fn handle_completed_downloads(
        app: &tauri::App,
        downloads: &dreamland_runtime::DownloadManager,
    ) {
        let handle = app.handle().clone();
        let mut completions = downloads.subscribe_completions();
        tauri::async_runtime::spawn(async move {
            // Android 13+ (API 33) requires the runtime POST_NOTIFICATIONS
            // prompt before any notification is actually shown -- the
            // manifest permission alone (auto-added by the plugin) is not
            // enough. Requesting once up front here; a no-op if already
            // granted or already denied.
            let _ = handle.notification().request_permission();
            while let Ok(record) = completions.recv().await {
                if let Some(target_path) = record.target_path.as_deref() {
                    if let Err(error) = publish_to_gallery(&handle, target_path).await {
                        eprintln!("could not publish download to Pictures: {error}");
                    }
                }
                let _ = handle
                    .notification()
                    .builder()
                    .title(format!("{} download ready", record.site.as_str()))
                    .body(format!("post #{}", record.post_id))
                    .show();
            }
        });
    }
}

/// Copies a completed download into the shared Pictures/Dreamland gallery
/// folder via MediaStore, so it shows up in the system Photos/Gallery app --
/// matching the vendored moebooru reference's own Pictures/$moeHost
/// convention (see docs/ANDROID-ASSET-AND-DETAIL-AUDIT.md). The original
/// copy under app-private storage is left in place; the rest of the app
/// (open-file, download dedup, thumbnails) keeps working from that path
/// unchanged, and this is a best-effort side effect on top of it.
#[cfg(mobile)]
async fn publish_to_gallery(app: &AppHandle, source_path: &str) -> Result<(), String> {
    let file_name = std::path::Path::new(source_path)
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "download has no file name".to_owned())?
        .to_owned();
    let bytes = tokio::fs::read(source_path)
        .await
        .map_err(|error| error.to_string())?;
    let api = app.android_fs_async();
    if !api
        .public_storage()
        .request_permission()
        .await
        .map_err(|error| error.to_string())?
    {
        return Err("gallery permission denied".to_owned());
    }
    api.public_storage()
        .write_new(
            None,
            PublicImageDir::Pictures,
            format!("Dreamland/{file_name}"),
            None,
            bytes,
        )
        .await
        .map(|_| ())
        .map_err(|error| error.to_string())
}

#[cfg(not(mobile))]
pub(crate) type ActivePlatform = Desktop;
#[cfg(mobile)]
pub(crate) type ActivePlatform = Mobile;
