use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

mod api;
mod config;
mod ui;

use api::ImagePost;
use config::AppConfig;

#[derive(Clone, Debug)]
pub struct AppState {
    pub images: Vec<ImagePost>,
    pub config: AppConfig,
    pub current_page: usize,
    pub loading: bool,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            images: Vec::new(),
            config: AppConfig::load().unwrap_or_default(),
            current_page: 1,
            loading: false,
        }
    }
}

fn main() {
    dioxus_desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    use_shared_state_provider(cx, AppState::default);
    
    render! {
        div { 
            id: "app",
            style: "width: 100vw; height: 100vh; display: flex; flex-direction: column; font-family: Arial, sans-serif;",
            ui::header::Header {}
            ui::gallery::Gallery {}
        }
    }
}
