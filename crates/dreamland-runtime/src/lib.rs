mod config;

pub use config::AppConfig;

pub async fn download_image(
    url: &str,
    identifier: &str,
    download_dir: &std::path::Path,
) -> anyhow::Result<std::path::PathBuf> {
    validate_image_identifier(identifier)?;
    tokio::fs::create_dir_all(download_dir).await?;
    let (temporary_path, content_type) = download_file(url, download_dir).await?;
    let extension = mime_to_extension(&content_type).unwrap_or("jpg");
    let filename = image_filename(identifier, extension)?;
    let final_path = download_dir.join(filename);

    tokio::fs::rename(&temporary_path, &final_path).await?;
    Ok(final_path)
}

async fn download_file(
    url: &str,
    download_dir: &std::path::Path,
) -> anyhow::Result<(std::path::PathBuf, String)> {
    let response = reqwest::Client::new()
        .get(url)
        .send()
        .await?
        .error_for_status()?;
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
}
