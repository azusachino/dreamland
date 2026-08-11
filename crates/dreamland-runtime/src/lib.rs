mod config;
mod sessions;

use dreamland_core::{NetworkPolicy, ProxyMode};

pub use config::{detect_proxy, AppConfig, ProxyDetection};
pub use sessions::{QuerySession, QuerySessionStore, SessionOperation};

pub async fn download_image(
    url: &str,
    identifier: &str,
    download_dir: &std::path::Path,
    network: &NetworkPolicy,
) -> anyhow::Result<std::path::PathBuf> {
    validate_image_identifier(identifier)?;
    tokio::fs::create_dir_all(download_dir).await?;
    let (temporary_path, content_type) = download_file(url, download_dir, network).await?;
    let extension = mime_to_extension(&content_type).unwrap_or("jpg");
    let filename = image_filename(identifier, extension)?;
    let final_path = download_dir.join(filename);

    tokio::fs::rename(&temporary_path, &final_path).await?;
    Ok(final_path)
}

async fn download_file(
    url: &str,
    download_dir: &std::path::Path,
    network: &NetworkPolicy,
) -> anyhow::Result<(std::path::PathBuf, String)> {
    let client = build_client(network)?;
    let mut retries = 0;
    let response = loop {
        match client.get(url).send().await {
            Ok(response) if response.status().is_success() => break response,
            Ok(response)
                if retryable_status(response.status()) && retries < network.max_retries =>
            {
                let delay = retry_delay(&response, network, retries);
                retries += 1;
                tokio::time::sleep(delay).await;
            }
            Ok(response) => break response.error_for_status()?,
            Err(error) if retries < network.max_retries => {
                let delay = network
                    .retry_delay_ms
                    .saturating_mul(2_u64.saturating_pow(retries.into()))
                    .min(network.max_retry_delay_ms);
                retries += 1;
                tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
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
    let temporary_path = download_dir.join(format!("temp_{}", uuid::Uuid::new_v4()));

    tokio::fs::write(&temporary_path, response.bytes().await?).await?;
    Ok((temporary_path, content_type))
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

fn validate_image_identifier(value: &str) -> anyhow::Result<()> {
    let value = value.trim();
    if value.is_empty() || !value.chars().all(|character| character.is_ascii_hexdigit()) {
        anyhow::bail!("image identifier is not a hexadecimal value");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
