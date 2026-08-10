use dreamland_provider_yandere::ImagePost;
use dreamland_runtime::AppConfig;

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
    dreamland_provider_yandere::fetch_images(&config.api_url, page)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn download_image(image: ImagePost) -> Result<String, String> {
    let config = AppConfig::load().map_err(|error| error.to_string())?;
    let final_path =
        dreamland_runtime::download_image(&image.file_url, &image.md5, &config.download_path)
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
