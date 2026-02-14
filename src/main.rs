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
        <style>
            @import url('https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&family=JetBrains+Mono:wght@400;500&display=swap');

            :root {
                --bg-base: #1e1e2e;
                --bg-sidebar: #181825;
                --bg-header: #11111b;
                --bg-surface: #313244;
                --bg-hover: #45475a;
                --text-main: #cdd6f4;
                --text-sub: #a6adc8;
                --accent-primary: #89b4fa;
                --accent-red: #f38ba8;
                --border-color: #45475a;
                --table-hover: rgba(137, 180, 250, 0.15);
            }

            html, body { 
                margin: 0; padding: 0; overflow: hidden; height: 100%; user-select: none; 
                font-family: 'Inter', sans-serif; background-color: var(--bg-base); color: var(--text-main);
            }

            .title-bar { 
                display: flex; justify-content: space-between; align-items: center; height: 38px; 
                background: var(--bg-header); border-bottom: 1px solid var(--border-color); flex-shrink: 0; 
                -webkit-app-region: drag;
            }
            
            .title-section-left { 
                flex: 1; display: flex; align-items: center; padding-left: 15px; 
                font-weight: 700; color: var(--text-main); font-size: 0.9em; letter-spacing: 0.5px;
            }
            .title-section-center { flex: 1; display: flex; justify-content: center; align-items: flex-start; height: 100%; }
            .title-section-right { flex: 1; display: flex; justify-content: flex-end; height: 100%; align-items: center; -webkit-app-region: no-drag; }

            .repo-status-box {
                display: flex; align-items: center; 
                background: linear-gradient(180deg, #2a2b3d 0%, #1e1e2e 100%);
                border-left: 1px solid var(--border-color);
                border-right: 1px solid var(--border-color);
                border-bottom: 1px solid var(--border-color);
                border-top: 3px solid var(--accent-red); 
                border-radius: 0 0 6px 6px;
                padding: 4px 20px;
                font-size: 0.85em;
                color: var(--text-main);
                box-shadow: 0 4px 12px rgba(0,0,0,0.3);
                height: 32px;
                margin-top: -1px;
                -webkit-app-region: no-drag;
            }
            .repo-status-box .repo-name { font-weight: 800; margin-right: 10px; color: #fff; }
            .repo-status-box .divider { opacity: 0.3; margin: 0 8px; font-weight: 100; }
            .repo-status-box .branch-icon { margin-right: 6px; color: var(--accent-red); transform: rotate(90deg); display: inline-block; }
            .repo-status-box .branch-name { font-family: 'JetBrains Mono', monospace; color: var(--accent-red); font-weight: 600; }

            .window-controls { display: flex; height: 100%; -webkit-app-region: no-drag; }
            .control-btn { 
                width: 46px; display: flex; align-items: center; justify-content: center; 
                cursor: pointer; transition: background 0.2s; height: 100%; 
                color: var(--text-sub); font-family: sans-serif; font-size: 0.9em; 
            }
            .control-btn:hover { background: var(--bg-surface); color: #fff; }
            .control-btn.close:hover { background: #e81123; color: white; }

            .menu-item { padding: 5px 12px; cursor: pointer; user-select: none; font-size: 0.9em; border-radius: 4px; transition: 0.2s; color: var(--text-sub); }
            .menu-item:hover { background-color: var(--bg-surface); color: #fff; }
            
            .menu-dropdown { 
                position: absolute; top: 100%; left: 0; background: rgba(30, 30, 46, 0.98); 
                border: 1px solid var(--border-color); box-shadow: 0 10px 30px rgba(0,0,0,0.5); 
                min-width: 220px; z-index: 3000; border-radius: 6px; padding: 5px; 
                backdrop-filter: blur(12px);
            }
            .dropdown-item { 
                padding: 8px 12px; cursor: pointer; display: flex; justify-content: space-between; 
                font-size: 0.9em; color: var(--text-main); border-radius: 4px; 
            }
            .dropdown-item:hover { background-color: var(--accent-primary); color: var(--bg-base); font-weight: 600; }
            .separator { border-top: 1px solid var(--border-color); margin: 4px 0; }

            .toolbar-btn { 
                padding: 6px 12px; border: 1px solid transparent; cursor: pointer; background: transparent; 
                display: flex; gap: 8px; align-items: center; font-size: 0.9em; color: var(--text-main); 
                border-radius: 6px; transition: all 0.2s; font-weight: 500;
            }
            .toolbar-btn:hover { background-color: var(--bg-surface); border: 1px solid var(--border-color); }

            .resizer { width: 4px; cursor: col-resize; background-color: var(--bg-base); flex-shrink: 0; border-left: 1px solid var(--border-color); }
            .resizer:hover { background-color: var(--accent-primary); }
            
            .context-menu { 
                position: fixed; background: rgba(30, 30, 46, 0.98); border: 1px solid var(--border-color); 
                box-shadow: 0 8px 32px rgba(0,0,0,0.6); z-index: 6000; padding: 6px; 
                min-width: 200px; font-size: 0.9em; border-radius: 6px; backdrop-filter: blur(12px); color: var(--text-main); 
            }
            
            .status-box { 
                background: linear-gradient(135deg, var(--accent-primary), #74c7ec); 
                color: var(--bg-base); padding: 4px 12px; border-radius: 12px; 
                font-size: 0.75em; font-weight: 800; white-space: nowrap; margin-right: 15px; 
                box-shadow: 0 0 10px rgba(137, 180, 250, 0.3);
            }
            
            .commit-graph-cell-svg {
                padding: 0 !important;
                width: 140px;
                min-width: 140px;
                overflow: hidden;
                position: relative;
            }
            .commit-graph-cell-svg svg { display: block; position: absolute; top: 0; left: 0; height: 100%; width: 100%; }
            
            input, textarea { 
                background: var(--bg-surface); border: 1px solid var(--border-color); color: white; 
                border-radius: 6px; outline: none; transition: border-color 0.2s; 
            }
            input:focus, textarea:focus { border-color: var(--accent-primary); }
            
            table { width: 100%; border-collapse: collapse; font-size: 0.9em; }
            thead tr { background: var(--bg-sidebar); position: sticky; top: 0; z-index: 10; }
            th { padding: 8px 10px; font-weight: 600; color: var(--text-sub); text-align: left; border-bottom: 1px solid var(--border-color); }
            td { padding: 8px 10px; border-bottom: 1px solid #2a2b3c; color: var(--text-main); }
            tr:hover td { background-color: var(--bg-surface); }

            /* Sidebar Specific Styles */
            .workspace-header {
                padding: 12px 15px 8px 15px; /* Oben, Rechts, Unten, Links */
                font-size: 0.75em;
                font-weight: 700;
                color: var(--text-sub);
                text-transform: uppercase;
                letter-spacing: 0.5px;
                border-bottom: 1px solid var(--border-color);
                margin-bottom: 5px;
            }

            .nav-item { 
                padding: 8px 15px;
                font-size: 0.9em;
                color: var(--text-main);
                cursor: pointer;
                transition: background-color 0.2s, color 0.2s;
                border-bottom: 1px solid var(--border-color); 
                display: flex; 
                align-items: center;
            }
            .nav-item:last-of-type {
                border-bottom: none; /* Keine Trennlinie für das letzte Element einer Gruppe */
            }
            .nav-item:hover { 
                background-color: var(--bg-hover);
            }
            .nav-item.active {
                background-color: var(--accent-primary);
                color: var(--bg-base);
                font-weight: 600;
            }
            .nav-item .badge {
                background-color: var(--bg-base);
                color: var(--accent-primary);
                padding: 2px 8px;
                border-radius: 10px;
                font-size: 0.7em;
                font-weight: 600;
                margin-left: auto;
            }

            ::-webkit-scrollbar { width: 8px; height: 8px; }
            ::-webkit-scrollbar-track { background: var(--bg-base); }
            ::-webkit-scrollbar-thumb { background: var(--border-color); border-radius: 4px; }
            ::-webkit-scrollbar-thumb:hover { background: var(--text-sub); }
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