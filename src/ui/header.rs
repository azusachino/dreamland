use dioxus::prelude::*;
use crate::{AppState, api};

#[inline_props]
pub fn Header(cx: Scope) -> Element {
    let app_state = use_shared_state::<AppState>(cx)?;
    let show_settings = use_state(cx, || false);
    
    let load_images = move |_| {
        let app_state = app_state.clone();
        cx.spawn(async move {
            app_state.write().loading = true;
            let current_page = app_state.read().current_page;
            let api_url = app_state.read().config.api_url.clone();
            
            match api::fetch_images(&api_url, current_page).await {
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
    };
    
    render! {
        header {
            style: "display: flex; justify-content: space-between; align-items: center; padding: 1rem; background: #2a2a2a; color: white;",
            div {
                h1 { "Dreamland Image Viewer" }
            }
            div {
                style: "display: flex; gap: 1rem; align-items: center;",
                button {
                    onclick: load_images,
                    disabled: app_state.read().loading,
                    style: "padding: 0.5rem 1rem; background: #4CAF50; color: white; border: none; border-radius: 4px; cursor: pointer;",
                    if app_state.read().loading { "Loading..." } else { "Load Images" }
                }
                button {
                    onclick: move |_| show_settings.set(!*show_settings.get()),
                    style: "padding: 0.5rem 1rem; background: #2196F3; color: white; border: none; border-radius: 4px; cursor: pointer;",
                    "Settings"
                }
            }
        }
        if *show_settings.get() {
            crate::ui::settings::Settings {
                on_close: move |_| show_settings.set(false)
            }
        }
    }
}