use dioxus::prelude::*;
use crate::{AppState, config::AppConfig};

#[inline_props]
pub fn Settings(cx: Scope, on_close: EventHandler<()>) -> Element {
    let app_state = use_shared_state::<AppState>(cx)?;
    let download_path = use_state(cx, || app_state.read().config.download_path.to_string_lossy().to_string());
    let api_url = use_state(cx, || app_state.read().config.api_url.clone());
    
    let save_settings = move |_| {
        let mut config = app_state.read().config.clone();
        config.download_path = download_path.get().clone().into();
        config.api_url = api_url.get().clone();
        
        if let Err(e) = config.save() {
            eprintln!("Failed to save config: {}", e);
        } else {
            app_state.write().config = config;
            on_close.call(());
        }
    };
    
    render! {
        div {
            style: "position: fixed; top: 0; left: 0; right: 0; bottom: 0; background: rgba(0,0,0,0.5); display: flex; justify-content: center; align-items: center; z-index: 1000;",
            div {
                style: "background: white; padding: 2rem; border-radius: 8px; min-width: 400px; max-width: 600px;",
                h2 {
                    style: "margin-top: 0; margin-bottom: 1.5rem;",
                    "Settings"
                }
                
                div {
                    style: "margin-bottom: 1.5rem;",
                    label {
                        style: "display: block; margin-bottom: 0.5rem; font-weight: bold;",
                        "API URL:"
                    }
                    input {
                        value: "{api_url}",
                        oninput: move |evt| api_url.set(evt.value.clone()),
                        style: "width: 100%; padding: 0.5rem; border: 1px solid #ddd; border-radius: 4px;",
                        placeholder: "https://yande.re/post.json"
                    }
                }
                
                div {
                    style: "margin-bottom: 1.5rem;",
                    label {
                        style: "display: block; margin-bottom: 0.5rem; font-weight: bold;",
                        "Download Path:"
                    }
                    input {
                        value: "{download_path}",
                        oninput: move |evt| download_path.set(evt.value.clone()),
                        style: "width: 100%; padding: 0.5rem; border: 1px solid #ddd; border-radius: 4px;",
                        placeholder: "/path/to/downloads"
                    }
                }
                
                div {
                    style: "display: flex; gap: 1rem; justify-content: flex-end;",
                    button {
                        onclick: move |_| on_close.call(()),
                        style: "padding: 0.5rem 1rem; background: #666; color: white; border: none; border-radius: 4px; cursor: pointer;",
                        "Cancel"
                    }
                    button {
                        onclick: save_settings,
                        style: "padding: 0.5rem 1rem; background: #4CAF50; color: white; border: none; border-radius: 4px; cursor: pointer;",
                        "Save"
                    }
                }
            }
        }
    }
}