#![allow(non_snake_case)]
mod i18n;
mod git;
mod app;
mod ui;

use dioxus::prelude::*;
use dioxus::desktop::{Config, WindowBuilder};
#[cfg(target_os = "windows")]
use dioxus::desktop::tao::platform::windows::WindowBuilderExtWindows;
use std::path::PathBuf; 
use std::fs;
use directories::BaseDirs;

pub fn get_config_path() -> Option<PathBuf> {
    if let Some(base_dirs) = BaseDirs::new() {
        let mut path = PathBuf::from(base_dirs.config_dir());
        path.push("gitamicus");
        path.push("gitamicus.conf");
        Some(path)
    }  else {
        None
    }
}

pub fn load_credentials() -> (String, String, String) {
    if let Some(config_path) = get_config_path() {
        if let Ok(content) = fs::read_to_string(&config_path) {
            let parts: Vec<&str> = content.split('\n').collect();
            let user = parts.get(0).map_or("", |s| s.trim()).to_string();
            let token = parts.get(1).map_or("", |s| s.trim()).to_string(); 
            let repo_path = parts.get(2).map_or("", |s| s.trim()).to_string(); 
            return (user, token, repo_path);
        }
    }
    
    (String::new(), String::new(), String::new())
}

pub fn save_credentials(user: &str, token: &str, repo_path: &str) -> Result<(), String> {
    if let Some(config_path) = get_config_path() {
        if let Some(parent_dir) = config_path.parent() {
            if let Err(e) = fs::create_dir_all(parent_dir) {
                return Err(format!("Fehler beim Erstellen des Konfigurationsverzeichnisses: {}", e));
            }
        }
        let data = format!("{}\n{}\n{}", user, token, repo_path);
        fs::write(&config_path, data).map_err(|e| format!("Fehler beim Schreiben der Anmeldedaten: {}", e))
    } else {
        Err("Konfigurationsverzeichnis konnte nicht ermittelt werden.".to_string())
    }
}

fn main() {
    let custom_head = r#"
        <link rel="stylesheet" href="app.css">
        <style>
            @import url('https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&family=JetBrains+Mono:wght@400;500&display=swap');
        </style>
        <script>
            document.addEventListener('contextmenu', event => event.preventDefault());
        </script>
    "#;

    let mut window = WindowBuilder::new()
        .with_title("GitAmicus")
        .with_always_on_top(false)
        .with_decorations(false)
        .with_resizable(true)
        .with_transparent(true);

    #[cfg(target_os = "windows")]
    {
        window = window.with_menu(0);
    }

    let config = Config::new()
        .with_custom_head(custom_head.to_string())
        .with_background_color((30, 30, 46, 255))
        .with_window(window);

    LaunchBuilder::desktop().with_cfg(config).launch(app::app);
}