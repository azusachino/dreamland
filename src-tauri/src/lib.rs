mod api;
mod config;

use api::ImagePost;
use config::AppConfig;

#[tauri::command]
fn load_config() -> Result<AppConfig, String> {
    AppConfig::load().map_err(|error| error.to_string())
}

#[tauri::command]
fn save_config(download_path: String, api_url: String) -> Result<AppConfig, String> {
    let config =
        AppConfig::from_user_input(download_path, api_url).map_err(|error| error.to_string())?;
    config.save().map_err(|error| error.to_string())?;
    Ok(config)
}

#[tauri::command]
async fn load_images(page: usize) -> Result<Vec<ImagePost>, String> {
    let config = AppConfig::load().map_err(|error| error.to_string())?;
    api::fetch_images(&config.api_url, page)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn download_image(image: ImagePost) -> Result<String, String> {
    api::validate_image_identifier(&image.md5).map_err(|error| error.to_string())?;
    let config = AppConfig::load().map_err(|error| error.to_string())?;
    tokio::fs::create_dir_all(&config.download_path)
        .await
        .map_err(|error| error.to_string())?;

    let (temporary_path, content_type) = api::download_file(&image.file_url, &config.download_path)
        .await
        .map_err(|error| error.to_string())?;
    let extension = api::mime_to_extension(&content_type).unwrap_or("jpg");
    let filename = api::image_filename(&image, extension).map_err(|error| error.to_string())?;
    let final_path = config.download_path.join(filename);

    tokio::fs::rename(&temporary_path, &final_path)
        .await
        .map_err(|error| error.to_string())?;
    Ok(final_path.display().to_string())
}

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            load_config,
            save_config,
            load_images,
            download_image
        ])
        .run(tauri::generate_context!())
        .expect("error while running Dreamland");
}
