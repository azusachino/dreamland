use dioxus::prelude::*;
use crate::{AppState, api::ImagePost};

#[inline_props]
pub fn Gallery(cx: Scope) -> Element {
    let app_state = use_shared_state::<AppState>(cx)?;
    
    render! {
        main {
            style: "flex: 1; overflow-y: auto; padding: 1rem;",
            if app_state.read().images.is_empty() {
                div {
                    style: "display: flex; justify-content: center; align-items: center; height: 100%; font-size: 1.2rem; color: #666;",
                    "Click 'Load Images' to fetch images from the API"
                }
            } else {
                div {
                    style: "display: grid; grid-template-columns: repeat(auto-fill, minmax(250px, 1fr)); gap: 1rem;",
                    for image in app_state.read().images.iter() {
                        ImageCard { image: image.clone() }
                    }
                }
                
                div {
                    style: "display: flex; justify-content: center; gap: 1rem; margin-top: 2rem;",
                    PaginationControls {}
                }
            }
        }
    }
}

#[inline_props]
fn ImageCard(cx: Scope, image: ImagePost) -> Element {
    let app_state = use_shared_state::<AppState>(cx)?;
    
    let download_image = move |_| {
        let file_url = image.file_url.clone();
        let download_path = app_state.read().config.download_path.clone();
        let filename = format!("{}.{}", image.md5, file_url.split('.').last().unwrap_or("jpg"));
        
        cx.spawn(async move {
            std::fs::create_dir_all(&download_path).ok();
            let full_path = download_path.join(filename);
            
            match crate::api::download_image(&file_url, &full_path.to_string_lossy()).await {
                Ok(_) => println!("Downloaded: {}", full_path.display()),
                Err(e) => eprintln!("Download failed: {}", e),
            }
        });
    };
    
    render! {
        div {
            style: "border: 1px solid #ddd; border-radius: 8px; overflow: hidden; background: white; box-shadow: 0 2px 4px rgba(0,0,0,0.1);",
            div {
                style: "position: relative;",
                img {
                    src: "{image.preview_url}",
                    alt: "Image preview",
                    style: "width: 100%; height: 200px; object-fit: cover;",
                    loading: "lazy"
                }
                div {
                    style: "position: absolute; top: 8px; right: 8px; background: rgba(0,0,0,0.7); color: white; padding: 4px 8px; border-radius: 4px; font-size: 0.8rem;",
                    "{image.width}x{image.height}"
                }
            }
            div {
                style: "padding: 1rem;",
                div {
                    style: "font-size: 0.9rem; color: #666; margin-bottom: 0.5rem; display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden;",
                    "Tags: {image.tags}"
                }
                div {
                    style: "display: flex; justify-content: space-between; align-items: center;",
                    span {
                        style: "font-size: 0.8rem; color: #888;",
                        "Rating: {image.rating}"
                    }
                    button {
                        onclick: download_image,
                        style: "background: #FF9800; color: white; border: none; padding: 0.5rem 1rem; border-radius: 4px; cursor: pointer; font-size: 0.8rem;",
                        "Download"
                    }
                }
            }
        }
    }
}

#[inline_props]
fn PaginationControls(cx: Scope) -> Element {
    let app_state = use_shared_state::<AppState>(cx)?;
    
    let prev_page = move |_| {
        if app_state.read().current_page > 1 {
            app_state.write().current_page -= 1;
            load_current_page(cx, app_state.clone());
        }
    };
    
    let next_page = move |_| {
        app_state.write().current_page += 1;
        load_current_page(cx, app_state.clone());
    };
    
    render! {
        div {
            style: "display: flex; gap: 1rem; align-items: center;",
            button {
                onclick: prev_page,
                disabled: app_state.read().current_page <= 1,
                style: "padding: 0.5rem 1rem; background: #2196F3; color: white; border: none; border-radius: 4px; cursor: pointer;",
                "Previous"
            }
            span {
                style: "font-weight: bold;",
                "Page {app_state.read().current_page}"
            }
            button {
                onclick: next_page,
                style: "padding: 0.5rem 1rem; background: #2196F3; color: white; border: none; border-radius: 4px; cursor: pointer;",
                "Next"
            }
        }
    }
}

fn load_current_page(cx: &ScopeState, app_state: UseSharedState<AppState>) {
    cx.spawn(async move {
        app_state.write().loading = true;
        let current_page = app_state.read().current_page;
        let api_url = app_state.read().config.api_url.clone();
        
        match crate::api::fetch_images(&api_url, current_page).await {
            Ok(images) => {
                app_state.write().images = images;
                app_state.write().loading = false;
            }
            Err(e) => {
                eprintln!("Failed to load images: {}", e);
                app_state.write().loading = false;
            }
        }
    });
}