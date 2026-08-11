mod config;
mod local_state;
mod sessions;

use dreamland_core::{NetworkPolicy, ProxyMode};
use tokio::io::AsyncWriteExt;

pub use config::{detect_proxy, AppConfig, ProxyDetection};
pub use local_state::{
    DownloadCancellation, DownloadManager, DownloadRecord, DownloadRequest, DownloadStatus,
    LocalStateStore,
};
pub use sessions::{QuerySession, QuerySessionStore, SessionOperation};

pub fn default_state_path() -> std::path::PathBuf {
    dirs::data_local_dir()
        .or_else(dirs::data_dir)
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("dreamland")
        .join("state.sqlite3")
}

pub fn default_cache_path() -> std::path::PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("dreamland")
        .join("downloads")
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
    validate_image_identifier(identifier)?;
    validate_path_component(site_id, "site")?;
    let posts_dir = download_dir.join(site_id).join("posts");
    tokio::fs::create_dir_all(&posts_dir).await?;
    match download_file(
        url,
        site_id,
        identifier,
        &posts_dir,
        cache_dir,
        network,
        cancellation,
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

async fn download_file(
    url: &str,
    site_id: &str,
    identifier: &str,
    target_dir: &std::path::Path,
    cache_dir: &std::path::Path,
    network: &NetworkPolicy,
    cancellation: &DownloadCancellation,
) -> anyhow::Result<DownloadFileOutcome> {
    validate_media_url(url)?;
    let client = build_client(network)?;
    let mut retries = 0;
    let response = loop {
        if cancellation.is_cancelled() {
            return Ok(DownloadFileOutcome::Cancelled);
        }
        match client.get(url).send().await {
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
    let final_path = target_dir.join(image_filename(identifier, extension)?);
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
    let mut builder = reqwest::Client::builder();
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
        assert_eq!(outcome, DownloadOutcome::ExistingTarget(target.clone()));
        assert_eq!(tokio::fs::read(target).await.unwrap(), b"original");
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
        manager.spawn_worker();
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
