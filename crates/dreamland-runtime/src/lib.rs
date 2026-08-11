mod config;
mod local_state;
mod sessions;

use dreamland_core::{NetworkPolicy, ProxyMode};
use tokio::io::AsyncWriteExt;

const USER_AGENT: &str = concat!("Dreamland/", env!("CARGO_PKG_VERSION"));
const DETAIL_IMAGE_EXTENSIONS: [&str; 7] = ["jpg", "jpeg", "png", "gif", "webp", "bmp", "avif"];

pub use config::{detect_proxy, AppConfig, ProxyDetection};
pub use local_state::{
    ArchiveRecord, ArchiveRequest, DownloadCancellation, DownloadManager, DownloadRecord,
    DownloadRequest, DownloadStatus, LocalStateStore,
};
pub use sessions::{QuerySession, QuerySessionStore, SessionOperation};

pub fn default_state_path() -> std::path::PathBuf {
    std::env::var_os("XDG_DATA_HOME")
        .map(std::path::PathBuf::from)
        .or_else(dirs::data_local_dir)
        .or_else(dirs::data_dir)
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("dreamland")
        .join("state.sqlite3")
}

pub fn default_cache_path() -> std::path::PathBuf {
    std::env::var_os("XDG_CACHE_HOME")
        .map(std::path::PathBuf::from)
        .or_else(dirs::cache_dir)
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("dreamland")
        .join("downloads")
}

pub fn default_log_path() -> std::path::PathBuf {
    std::env::var_os("XDG_STATE_HOME")
        .map(std::path::PathBuf::from)
        .or_else(dirs::data_local_dir)
        .or_else(dirs::data_dir)
        .or_else(dirs::cache_dir)
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("dreamland")
        .join("logs")
        .join("dreamland.log")
}

fn log_detail_event(event: &str) {
    let path = default_log_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        use std::io::Write;
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let _ = writeln!(file, "{timestamp} detail_image {event}");
    }
}

pub fn log_detail_failure(site_id: &str, post_id: &str, error: &str) {
    let error = error.replace(['\r', '\n'], " ");
    log_detail_event(&format!(
        "failure site={site_id} post={post_id} error={error}"
    ));
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DownloadOutcome {
    Completed(std::path::PathBuf),
    ExistingTarget(std::path::PathBuf),
    Cancelled,
}

pub async fn download_image(
    url: &str,
    site_id: &str,
    identifier: &str,
    download_dir: &std::path::Path,
    cache_dir: &std::path::Path,
    network: &NetworkPolicy,
    cancellation: &DownloadCancellation,
) -> anyhow::Result<DownloadOutcome> {
    download_image_with_detail_cache(
        url,
        site_id,
        identifier,
        download_dir,
        cache_dir,
        None,
        network,
        cancellation,
    )
    .await
}

pub async fn download_image_with_detail_cache(
    url: &str,
    site_id: &str,
    identifier: &str,
    download_dir: &std::path::Path,
    cache_dir: &std::path::Path,
    detail_cache_root: Option<&std::path::Path>,
    network: &NetworkPolicy,
    cancellation: &DownloadCancellation,
) -> anyhow::Result<DownloadOutcome> {
    validate_image_identifier(identifier)?;
    validate_path_component(site_id, "site")?;
    let posts_dir = download_dir.join(site_id).join("posts");
    tokio::fs::create_dir_all(&posts_dir).await?;
    if let Some(path) = existing_image_path(&posts_dir, identifier).await? {
        return Ok(DownloadOutcome::ExistingTarget(path));
    }
    if let Some(detail_cache_root) = detail_cache_root {
        if let Some(path) =
            existing_image_path(&detail_cache_root.join(site_id).join("posts"), identifier).await?
        {
            return promote_cached_image(
                &path,
                identifier,
                &posts_dir,
                cache_dir,
                site_id,
                cancellation,
            )
            .await;
        }
    }
    match download_file(
        url,
        site_id,
        identifier,
        &posts_dir,
        cache_dir,
        network,
        cancellation,
        None,
        None,
    )
    .await?
    {
        DownloadFileOutcome::ExistingTarget(path) => Ok(DownloadOutcome::ExistingTarget(path)),
        DownloadFileOutcome::Cancelled => Ok(DownloadOutcome::Cancelled),
        DownloadFileOutcome::Staged {
            temporary_path,
            final_path,
        } => {
            if cancellation.is_cancelled() {
                let _ = tokio::fs::remove_file(&temporary_path).await;
                return Ok(DownloadOutcome::Cancelled);
            }
            if tokio::fs::try_exists(&final_path).await? {
                let _ = tokio::fs::remove_file(&temporary_path).await;
                return Ok(DownloadOutcome::ExistingTarget(final_path));
            }
            tokio::fs::rename(&temporary_path, &final_path).await?;
            Ok(DownloadOutcome::Completed(final_path))
        }
    }
}

async fn promote_cached_image(
    cached_path: &std::path::Path,
    identifier: &str,
    target_dir: &std::path::Path,
    cache_dir: &std::path::Path,
    site_id: &str,
    cancellation: &DownloadCancellation,
) -> anyhow::Result<DownloadOutcome> {
    if cancellation.is_cancelled() {
        return Ok(DownloadOutcome::Cancelled);
    }
    let extension = cached_path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("jpg");
    let final_path = target_dir.join(image_filename(identifier, extension)?);
    if tokio::fs::try_exists(&final_path).await? {
        return Ok(DownloadOutcome::ExistingTarget(final_path));
    }
    let staging_dir = cache_dir.join(site_id);
    tokio::fs::create_dir_all(&staging_dir).await?;
    let temporary_path = staging_dir.join(format!("{}.{}.part", identifier, uuid::Uuid::new_v4()));
    if let Err(error) = tokio::fs::copy(cached_path, &temporary_path).await {
        let _ = tokio::fs::remove_file(&temporary_path).await;
        return Err(error.into());
    }
    if cancellation.is_cancelled() {
        let _ = tokio::fs::remove_file(&temporary_path).await;
        return Ok(DownloadOutcome::Cancelled);
    }
    if tokio::fs::try_exists(&final_path).await? {
        let _ = tokio::fs::remove_file(&temporary_path).await;
        return Ok(DownloadOutcome::ExistingTarget(final_path));
    }
    tokio::fs::rename(&temporary_path, &final_path).await?;
    Ok(DownloadOutcome::Completed(final_path))
}

pub async fn download_archive(
    url: &str,
    site_id: &str,
    pool_id: &str,
    pool_name: &str,
    download_dir: &std::path::Path,
    cache_dir: &std::path::Path,
    network: &NetworkPolicy,
    cookie_header: &str,
    cancellation: &DownloadCancellation,
) -> anyhow::Result<DownloadOutcome> {
    validate_path_component(site_id, "site")?;
    if pool_id.is_empty() || !pool_id.chars().all(|value| value.is_ascii_digit()) {
        anyhow::bail!("pool id must be numeric");
    }
    let pools_dir = download_dir.join(site_id).join("pools");
    tokio::fs::create_dir_all(&pools_dir).await?;
    let filename = archive_filename(pool_id, pool_name)?;
    match download_file(
        url,
        site_id,
        pool_id,
        &pools_dir,
        cache_dir,
        network,
        cancellation,
        Some(cookie_header),
        Some(&filename),
    )
    .await?
    {
        DownloadFileOutcome::ExistingTarget(path) => Ok(DownloadOutcome::ExistingTarget(path)),
        DownloadFileOutcome::Cancelled => Ok(DownloadOutcome::Cancelled),
        DownloadFileOutcome::Staged {
            temporary_path,
            final_path,
        } => {
            if cancellation.is_cancelled() {
                let _ = tokio::fs::remove_file(&temporary_path).await;
                return Ok(DownloadOutcome::Cancelled);
            }
            if tokio::fs::try_exists(&final_path).await? {
                let _ = tokio::fs::remove_file(&temporary_path).await;
                return Ok(DownloadOutcome::ExistingTarget(final_path));
            }
            tokio::fs::rename(&temporary_path, &final_path).await?;
            Ok(DownloadOutcome::Completed(final_path))
        }
    }
}

enum DownloadFileOutcome {
    Staged {
        temporary_path: std::path::PathBuf,
        final_path: std::path::PathBuf,
    },
    ExistingTarget(std::path::PathBuf),
    Cancelled,
}

/// Fetch a detail image through the Rust client and retain it in the local
/// cache. The frontend never needs to request the remote file host directly.
pub async fn cache_detail_image(
    url: &str,
    site_id: &str,
    post_id: &str,
    network: &NetworkPolicy,
) -> anyhow::Result<std::path::PathBuf> {
    let cache_root = detail_cache_path();
    cache_detail_image_at(url, site_id, post_id, &cache_root, network).await
}

pub async fn find_cached_detail_image_at(
    site_id: &str,
    post_id: &str,
    cache_root: &std::path::Path,
) -> anyhow::Result<Option<std::path::PathBuf>> {
    validate_image_identifier(post_id)?;
    validate_path_component(site_id, "site")?;
    existing_image_path(&cache_root.join(site_id).join("posts"), post_id).await
}

pub async fn cache_detail_image_at(
    url: &str,
    site_id: &str,
    post_id: &str,
    cache_root: &std::path::Path,
    network: &NetworkPolicy,
) -> anyhow::Result<std::path::PathBuf> {
    validate_image_identifier(post_id)?;
    validate_path_component(site_id, "site")?;
    let posts_dir = cache_root.join(site_id).join("posts");
    if let Some(path) = existing_image_path(&posts_dir, post_id).await? {
        log_detail_event(&format!(
            "cache_hit site={site_id} post={post_id} path={}",
            path.display()
        ));
        return Ok(path);
    }
    let staging_root = cache_root.with_file_name("detail-staging");
    log_detail_event(&format!("cache_miss site={site_id} post={post_id}"));
    let result = download_image(
        url,
        site_id,
        post_id,
        &cache_root,
        &staging_root,
        network,
        &crate::DownloadCancellation::default(),
    )
    .await?;
    match result {
        DownloadOutcome::Completed(path) | DownloadOutcome::ExistingTarget(path) => {
            log_detail_event(&format!(
                "downloaded site={site_id} post={post_id} path={}",
                path.display()
            ));
            Ok(path)
        }
        DownloadOutcome::Cancelled => anyhow::bail!("detail image request was cancelled"),
    }
}

fn detail_cache_path() -> std::path::PathBuf {
    default_cache_path().with_file_name("detail")
}

async fn existing_image_path(
    posts_dir: &std::path::Path,
    identifier: &str,
) -> anyhow::Result<Option<std::path::PathBuf>> {
    for extension in DETAIL_IMAGE_EXTENSIONS {
        let path = posts_dir.join(format!("{identifier}.{extension}"));
        if tokio::fs::try_exists(&path).await? {
            return Ok(Some(path));
        }
    }
    Ok(None)
}

async fn download_file(
    url: &str,
    site_id: &str,
    identifier: &str,
    target_dir: &std::path::Path,
    cache_dir: &std::path::Path,
    network: &NetworkPolicy,
    cancellation: &DownloadCancellation,
    cookie_header: Option<&str>,
    filename: Option<&str>,
) -> anyhow::Result<DownloadFileOutcome> {
    validate_media_url(url)?;
    let client = build_client(network)?;
    let mut retries = 0;
    let response = loop {
        if cancellation.is_cancelled() {
            return Ok(DownloadFileOutcome::Cancelled);
        }
        let mut request = client.get(url);
        if let Some(cookie_header) = cookie_header {
            request = request.header(reqwest::header::COOKIE, cookie_header);
        }
        match request.send().await {
            Ok(response) if response.status().is_success() => {
                if !matches!(response.url().scheme(), "http" | "https") {
                    anyhow::bail!("media redirect used an unsafe URL scheme");
                }
                break response;
            }
            Ok(response)
                if retryable_status(response.status()) && retries < network.max_retries =>
            {
                let delay = retry_delay(&response, network, retries);
                retries += 1;
                if cancellation.sleep_or_cancel(delay).await {
                    return Ok(DownloadFileOutcome::Cancelled);
                }
            }
            Ok(response) => break response.error_for_status()?,
            Err(error) if retries < network.max_retries => {
                let delay = network
                    .retry_delay_ms
                    .saturating_mul(2_u64.saturating_pow(retries.into()))
                    .min(network.max_retry_delay_ms);
                retries += 1;
                if cancellation
                    .sleep_or_cancel(std::time::Duration::from_millis(delay))
                    .await
                {
                    return Ok(DownloadFileOutcome::Cancelled);
                }
                let _ = error;
            }
            Err(error) => return Err(error.into()),
        }
    };
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("image/jpeg")
        .split(';')
        .next()
        .unwrap_or("image/jpeg")
        .to_string();
    let extension = mime_to_extension(&content_type).unwrap_or("jpg");
    let final_path = target_dir.join(
        filename
            .map(str::to_owned)
            .unwrap_or(image_filename(identifier, extension)?),
    );
    if tokio::fs::try_exists(&final_path).await? {
        return Ok(DownloadFileOutcome::ExistingTarget(final_path));
    }

    let cache_dir = cache_dir.join(site_id);
    tokio::fs::create_dir_all(&cache_dir).await?;
    let temporary_path = cache_dir.join(format!("{}.{}.part", identifier, uuid::Uuid::new_v4()));
    let mut file = tokio::fs::File::create(&temporary_path).await?;
    let write_result: anyhow::Result<()> = async {
        let mut response = response;
        while let Some(chunk) = response.chunk().await? {
            if cancellation.is_cancelled() {
                anyhow::bail!("download cancelled");
            }
            file.write_all(&chunk).await?;
        }
        file.flush().await?;
        Ok(())
    }
    .await;
    if let Err(error) = write_result {
        let _ = tokio::fs::remove_file(&temporary_path).await;
        if cancellation.is_cancelled() {
            return Ok(DownloadFileOutcome::Cancelled);
        }
        return Err(error);
    }
    Ok(DownloadFileOutcome::Staged {
        temporary_path,
        final_path,
    })
}

fn build_client(network: &NetworkPolicy) -> anyhow::Result<reqwest::Client> {
    let mut builder = reqwest::Client::builder().user_agent(USER_AGENT);
    match &network.proxy {
        ProxyMode::Auto => {}
        ProxyMode::Direct => {
            builder = builder.no_proxy();
        }
        ProxyMode::Manual { url } => {
            builder = builder.proxy(reqwest::Proxy::all(url)?);
        }
    }
    Ok(builder.build()?)
}

fn retryable_status(status: reqwest::StatusCode) -> bool {
    matches!(status.as_u16(), 429 | 500 | 502 | 503 | 504)
}

fn retry_delay(
    response: &reqwest::Response,
    network: &NetworkPolicy,
    retries: u8,
) -> std::time::Duration {
    let hinted = response
        .headers()
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
        .map(|seconds| seconds.saturating_mul(1000));
    let fallback = network
        .retry_delay_ms
        .saturating_mul(2_u64.saturating_pow(retries.into()));
    std::time::Duration::from_millis(hinted.unwrap_or(fallback).min(network.max_retry_delay_ms))
}

fn mime_to_extension(content_type: &str) -> Option<&str> {
    match content_type {
        "image/jpeg" => Some("jpg"),
        "image/png" => Some("png"),
        "image/gif" => Some("gif"),
        "image/webp" => Some("webp"),
        "image/bmp" => Some("bmp"),
        "image/svg+xml" => Some("svg"),
        _ => None,
    }
}

fn image_filename(identifier: &str, extension: &str) -> anyhow::Result<String> {
    validate_image_identifier(identifier)?;
    Ok(format!("{}.{}", identifier.trim(), extension))
}

fn archive_filename(pool_id: &str, pool_name: &str) -> anyhow::Result<String> {
    if pool_id.is_empty() || !pool_id.chars().all(|value| value.is_ascii_digit()) {
        anyhow::bail!("pool id must be numeric");
    }
    let mut slug = String::new();
    for character in pool_name.trim().chars() {
        if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
            slug.push(character.to_ascii_lowercase());
        } else if !slug.ends_with('-') {
            slug.push('-');
        }
        if slug.len() >= 50 {
            break;
        }
    }
    let slug = slug.trim_matches('-');
    let suffix = if slug.is_empty() { "pool" } else { slug };
    Ok(format!("pool-{pool_id}_{suffix}.zip"))
}

fn validate_media_url(value: &str) -> anyhow::Result<()> {
    let url = reqwest::Url::parse(value)?;
    if !matches!(url.scheme(), "http" | "https") {
        anyhow::bail!("media URL must use http or https");
    }
    Ok(())
}

fn validate_path_component(value: &str, label: &str) -> anyhow::Result<()> {
    if value.is_empty()
        || value.len() > 80
        || !value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
    {
        anyhow::bail!("invalid {label} path component");
    }
    Ok(())
}

fn validate_image_identifier(value: &str) -> anyhow::Result<()> {
    let value = value.trim();
    if value.is_empty() || !value.chars().all(|character| character.is_ascii_hexdigit()) {
        anyhow::bail!("image identifier is not a hexadecimal value");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    use dreamland_core::{NetworkPolicy, Post, PostRef, Rating, SiteId};

    use super::*;

    fn image_server() -> (String, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0_u8; 512];
            let _ = stream.read(&mut request);
            stream
                .write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Type: image/png\r\nContent-Length: 4\r\nConnection: close\r\n\r\nPNG!",
                )
                .unwrap();
        });
        (format!("http://{address}/image"), handle)
    }

    #[test]
    fn mime_types_map_to_extensions() {
        assert_eq!(mime_to_extension("image/jpeg"), Some("jpg"));
        assert_eq!(mime_to_extension("image/unknown"), None);
    }

    #[test]
    fn image_filename_rejects_path_traversal() {
        assert!(image_filename("../dreamland", "jpg").is_err());
    }

    #[test]
    fn only_transient_download_statuses_are_retried() {
        assert!(retryable_status(reqwest::StatusCode::TOO_MANY_REQUESTS));
        assert!(retryable_status(reqwest::StatusCode::BAD_GATEWAY));
        assert!(!retryable_status(reqwest::StatusCode::NOT_FOUND));
    }

    #[tokio::test]
    async fn download_stages_to_cache_then_atomically_commits_canonical_path() {
        let root = std::env::temp_dir().join(format!("dreamland-root-{}", uuid::Uuid::new_v4()));
        let cache = std::env::temp_dir().join(format!("dreamland-cache-{}", uuid::Uuid::new_v4()));
        let (url, server) = image_server();
        let outcome = download_image(
            &url,
            "yandere",
            "123",
            &root,
            &cache,
            &NetworkPolicy {
                proxy: ProxyMode::Direct,
                ..NetworkPolicy::default()
            },
            &DownloadCancellation::default(),
        )
        .await
        .unwrap();
        server.join().unwrap();
        let path = root.join("yandere/posts/123.png");
        assert_eq!(outcome, DownloadOutcome::Completed(path.clone()));
        assert_eq!(tokio::fs::read(&path).await.unwrap(), b"PNG!");
        assert!(tokio::fs::read_dir(cache.join("yandere"))
            .await
            .unwrap()
            .next_entry()
            .await
            .unwrap()
            .is_none());
        let _ = tokio::fs::remove_dir_all(root).await;
        let _ = tokio::fs::remove_dir_all(cache).await;
    }

    #[tokio::test]
    async fn existing_canonical_target_is_terminal_without_overwrite() {
        let root = std::env::temp_dir().join(format!("dreamland-root-{}", uuid::Uuid::new_v4()));
        let cache = std::env::temp_dir().join(format!("dreamland-cache-{}", uuid::Uuid::new_v4()));
        let target = root.join("yandere/posts/123.png");
        tokio::fs::create_dir_all(target.parent().unwrap())
            .await
            .unwrap();
        tokio::fs::write(&target, b"original").await.unwrap();
        let outcome = download_image(
            "http://127.0.0.1:1/unreachable.png",
            "yandere",
            "123",
            &root,
            &cache,
            &NetworkPolicy {
                proxy: ProxyMode::Direct,
                ..NetworkPolicy::default()
            },
            &DownloadCancellation::default(),
        )
        .await
        .unwrap();
        assert_eq!(outcome, DownloadOutcome::ExistingTarget(target.clone()));
        assert_eq!(tokio::fs::read(target).await.unwrap(), b"original");
        let _ = tokio::fs::remove_dir_all(root).await;
        let _ = tokio::fs::remove_dir_all(cache).await;
    }

    #[tokio::test]
    async fn existing_image_target_does_not_require_remote_request() {
        let root = std::env::temp_dir().join(format!("dreamland-root-{}", uuid::Uuid::new_v4()));
        let cache = std::env::temp_dir().join(format!("dreamland-cache-{}", uuid::Uuid::new_v4()));
        let target = root.join("yandere/posts/123.jpg");
        tokio::fs::create_dir_all(target.parent().unwrap())
            .await
            .unwrap();
        tokio::fs::write(&target, b"cached").await.unwrap();

        let outcome = download_image(
            "http://127.0.0.1:1/unreachable.jpg",
            "yandere",
            "123",
            &root,
            &cache,
            &NetworkPolicy {
                proxy: ProxyMode::Direct,
                ..NetworkPolicy::default()
            },
            &DownloadCancellation::default(),
        )
        .await
        .unwrap();

        assert_eq!(outcome, DownloadOutcome::ExistingTarget(target.clone()));
        assert_eq!(tokio::fs::read(target).await.unwrap(), b"cached");
        let _ = tokio::fs::remove_dir_all(root).await;
        let _ = tokio::fs::remove_dir_all(cache).await;
    }

    #[tokio::test]
    async fn download_promotes_detail_cache_without_remote_request() {
        let root = std::env::temp_dir().join(format!("dreamland-root-{}", uuid::Uuid::new_v4()));
        let cache = std::env::temp_dir().join(format!("dreamland-cache-{}", uuid::Uuid::new_v4()));
        let detail =
            std::env::temp_dir().join(format!("dreamland-detail-{}", uuid::Uuid::new_v4()));
        let cached = detail.join("yandere/posts/123.jpg");
        tokio::fs::create_dir_all(cached.parent().unwrap())
            .await
            .unwrap();
        tokio::fs::write(&cached, b"detail-cache").await.unwrap();

        let outcome = download_image_with_detail_cache(
            "http://127.0.0.1:1/unreachable.jpg",
            "yandere",
            "123",
            &root,
            &cache,
            Some(&detail),
            &NetworkPolicy {
                proxy: ProxyMode::Direct,
                ..NetworkPolicy::default()
            },
            &DownloadCancellation::default(),
        )
        .await
        .unwrap();

        let target = root.join("yandere/posts/123.jpg");
        assert_eq!(outcome, DownloadOutcome::Completed(target.clone()));
        assert_eq!(tokio::fs::read(target).await.unwrap(), b"detail-cache");
        assert!(!cache.join("yandere/123.jpg").exists());
        let _ = tokio::fs::remove_dir_all(root).await;
        let _ = tokio::fs::remove_dir_all(cache).await;
        let _ = tokio::fs::remove_dir_all(detail).await;
    }

    #[tokio::test]
    async fn cached_detail_image_is_found_without_remote_url() {
        let detail =
            std::env::temp_dir().join(format!("dreamland-detail-{}", uuid::Uuid::new_v4()));
        let cached = detail.join("yandere/posts/123.jpg");
        tokio::fs::create_dir_all(cached.parent().unwrap())
            .await
            .unwrap();
        tokio::fs::write(&cached, b"detail-cache").await.unwrap();

        let found = find_cached_detail_image_at("yandere", "123", &detail)
            .await
            .unwrap();

        assert_eq!(found, Some(cached));
        let _ = tokio::fs::remove_dir_all(detail).await;
    }

    #[tokio::test]
    async fn archive_download_uses_pool_layout_and_safe_name() {
        let root = std::env::temp_dir().join(format!("dreamland-root-{}", uuid::Uuid::new_v4()));
        let cache = std::env::temp_dir().join(format!("dreamland-cache-{}", uuid::Uuid::new_v4()));
        let (url, server) = image_server();
        let outcome = download_archive(
            &url,
            "yandere",
            "42",
            "Art Book / 2026",
            &root,
            &cache,
            &NetworkPolicy {
                proxy: ProxyMode::Direct,
                ..NetworkPolicy::default()
            },
            "_session=transient",
            &DownloadCancellation::default(),
        )
        .await
        .unwrap();
        server.join().unwrap();
        let path = root.join("yandere/pools/pool-42_art-book-2026.zip");
        assert_eq!(outcome, DownloadOutcome::Completed(path.clone()));
        assert_eq!(tokio::fs::read(path).await.unwrap(), b"PNG!");
        let _ = tokio::fs::remove_dir_all(root).await;
        let _ = tokio::fs::remove_dir_all(cache).await;
    }

    #[tokio::test]
    async fn queue_worker_returns_immediately_and_records_completion() {
        let state =
            std::env::temp_dir().join(format!("dreamland-state-{}.sqlite3", uuid::Uuid::new_v4()));
        let root = std::env::temp_dir().join(format!("dreamland-root-{}", uuid::Uuid::new_v4()));
        let cache = std::env::temp_dir().join(format!("dreamland-cache-{}", uuid::Uuid::new_v4()));
        let (url, server) = image_server();
        let manager = DownloadManager::open(
            &state,
            &cache,
            NetworkPolicy {
                proxy: ProxyMode::Direct,
                ..NetworkPolicy::default()
            },
        )
        .unwrap();
        tokio::spawn(manager.worker());
        let queued = manager
            .enqueue(DownloadRequest {
                site: SiteId::new("yandere"),
                post_id: "123".to_owned(),
                variant: dreamland_core::MediaVariant::Full,
                source_url: url,
                download_root: root.clone(),
                metadata: Post {
                    post: PostRef {
                        site: SiteId::new("yandere"),
                        id: "123".to_owned(),
                    },
                    tags: vec![],
                    author: None,
                    creator_id: None,
                    md5: None,
                    source: None,
                    parent_id: None,
                    has_children: false,
                    created_at: None,
                    width: None,
                    height: None,
                    rating: Rating::Safe,
                    score: None,
                    preview_url: None,
                    sample_url: None,
                    full_url: None,
                    file_size: None,
                },
            })
            .await
            .unwrap();
        assert_eq!(queued.status, DownloadStatus::Queued);
        let mut completed = false;
        for _ in 0..50 {
            if manager
                .records(10)
                .await
                .unwrap()
                .iter()
                .any(|record| record.status == DownloadStatus::Completed)
            {
                completed = true;
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        server.join().unwrap();
        assert!(completed);
        assert!(root.join("yandere/posts/123.png").exists());
        let _ = tokio::fs::remove_file(state).await;
        let _ = tokio::fs::remove_dir_all(root).await;
        let _ = tokio::fs::remove_dir_all(cache).await;
    }
}
