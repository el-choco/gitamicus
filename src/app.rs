use dioxus::prelude::*;
use crate::dioxus_elements::input_data::MouseButton;
use crate::i18n::I18nService;
use crate::git::{GitHandler, GRAPH_COLORS};
use crate::{load_credentials, save_credentials};
use sys_locale::get_locale;
use std::path::Path;
use std::process::Command;
use rfd;

pub fn app() -> Element {
    let i18n_service = use_signal(|| {
        let system_lang = get_locale().unwrap_or_else(|| "en-US".to_string());
        if system_lang.starts_with("de") { I18nService::new("de-DE") } else { I18nService::new("en-US") }
    });

    let (saved_user, saved_token, saved_repo_path) = load_credentials();

    let mut repo_path = use_signal(|| {
        if saved_repo_path.is_empty() {
            ".".to_string()
        } else {
            saved_repo_path
        }
    });
    let mut refresh_trigger = use_signal(|| 0);
    let mut commit_msg = use_signal(|| "".to_string());
    let mut selected_file = use_signal(|| None::<String>);
    let mut selected_commit = use_signal(|| None::<String>);
    let mut view_mode = use_signal(|| "local".to_string());
    let mut right_panel_tab = use_signal(|| "commit".to_string());
    
    let mut git_user = use_signal(|| saved_user);
    let mut git_token = use_signal(|| saved_token);
    let mut status_msg = use_signal(|| "ready".to_string());
    
    let mut show_push_menu = use_signal(|| false);
    let mut active_menu = use_signal(|| None::<String>);
    
    let mut commit_search = use_signal(|| "".to_string());
    let file_search = use_signal(|| "".to_string());
    let mut new_branch_name = use_signal(|| "".to_string());
    let mut context_menu_pos = use_signal(|| None::<(f64, f64, String, String)>);

    let mut show_reword_modal = use_signal(|| false);
    let mut reword_input = use_signal(|| "".to_string());
    let mut show_author_modal = use_signal(|| false);
    let mut author_name_input = use_signal(|| "".to_string());
    let mut author_email_input = use_signal(|| "".to_string());
    
    let mut show_clone_modal = use_signal(|| false);
    let mut clone_url_input = use_signal(|| "".to_string());
    let mut show_branch_modal = use_signal(|| false);
    let mut modal_branch_name = use_signal(|| "".to_string());
    let mut show_tag_modal = use_signal(|| false);
    let mut modal_tag_name = use_signal(|| "".to_string());
    
    let mut show_settings_modal = use_signal(|| false);

    let mut zoom_level = use_signal(|| 1.0);
    let mut view_all_commits = use_signal(|| false);
    
    let mut sidebar_width = use_signal(|| 250.0);
    let mut right_panel_width = use_signal(|| 500.0);
    let mut dragging_sidebar = use_signal(|| false);
    let mut dragging_right = use_signal(|| false);

    let i18n = i18n_service.read();
    let current_path = repo_path.read();
    let _ = refresh_trigger.read();
    
    let current_branch = GitHandler::get_current_branch(&current_path).unwrap_or_else(|e| i18n.translate(&e));
    let branches = GitHandler::get_branches(&current_path).unwrap_or_default();
    let remote_branches = GitHandler::get_remote_branches(&current_path).unwrap_or_default();
    let tags = GitHandler::get_tags(&current_path).unwrap_or_default();
    
    let repo_name = Path::new(&*current_path).file_name().and_then(|n| n.to_str()).unwrap_or("GitAmicus").to_string();

    let commits_raw: Vec<(String, String, String, String, Vec<String>)> = GitHandler::get_latest_commits_full(&current_path, 100, *view_all_commits.read()).unwrap_or_default();
    
    // Generate the full graph layout from the raw (unfiltered) commits
    let graph_nodes = crate::git::graph::generate_graph(&commits_raw);
    let graph_map: std::collections::HashMap<String, crate::git::graph::GraphNode> =
        graph_nodes.into_iter().map(|node| (node.sha.clone(), node)).collect();

    let commits: Vec<_> = commits_raw.clone().into_iter()
        .filter(|c| c.1.to_lowercase().contains(&commit_search.read().to_lowercase()))
        .collect();

    let changes_raw = GitHandler::get_status(&current_path).unwrap_or_default();
    
    let staged_files: Vec<String> = changes_raw.iter()
        .filter(|(_, s)| s == "staged")
        .map(|(p, _)| p.clone())
        .filter(|f| f.to_lowercase().contains(&file_search.read().to_lowercase()))
        .collect();

    let unstaged_files: Vec<String> = changes_raw.iter()
        .filter(|(_, s)| s == "unstaged")
        .map(|(p, _)| p.clone())
        .filter(|f| f.to_lowercase().contains(&file_search.read().to_lowercase()))
        .collect();

    let commit_files = if let Some(ref sha) = *selected_commit.read() {
        GitHandler::get_commit_files(&current_path, sha).unwrap_or_default()
    } else { Vec::new() };

    let diff_content = if *view_mode.read() == "local" {
        if let Some(ref file) = *selected_file.read() {
            GitHandler::get_file_diff(&current_path, file).unwrap_or_default()
        } else { "".to_string() }
    } else {
        if let Some(ref sha) = *selected_commit.read() {
            if let Some(ref file) = *selected_file.read() {
                GitHandler::get_commit_file_diff(&current_path, sha, file).unwrap_or_default()
            } else { "".to_string() }
        } else { "".to_string() }
    };

    let commit_details = if let Some(ref sha) = *selected_commit.read() {
        GitHandler::get_commit_details(&current_path, sha).ok()
    } else { None };

    rsx! {
        div {
            style: "display: flex; flex-direction: column; width: 100vw; height: 100vh; overflow: hidden; background: var(--bg-base); color: var(--text-main); zoom: {zoom_level};",
            prevent_default: "oncontextmenu",
            oncontextmenu: move |_| {}, 
            onmousedown: move |_| {
                context_menu_pos.set(None);
                active_menu.set(None);
            },
            onmouseup: move |_| {
                dragging_sidebar.set(false);
                dragging_right.set(false);
            },
            onmousemove: move |evt| {
                if *dragging_sidebar.read() {
                    let x = evt.page_coordinates().x;
                    if x > 100.0 && x < 800.0 { sidebar_width.set(x); }
                }
                if *dragging_right.read() {
                    let window_size = dioxus::desktop::window().inner_size();
                    let total_w = window_size.width as f64;
                    let x = evt.page_coordinates().x;
                    let new_w = total_w - x;
                    if new_w > 200.0 && new_w < 1200.0 { right_panel_width.set(new_w); }
                }
            },

            div { class: "title-bar",
                onmousedown: |e| {
                    if e.held_buttons().contains(MouseButton::Primary) {
                        dioxus::desktop::window().drag();
                    }
                },
                div { class: "title-section-left", 
                    span { style: "color: var(--accent-primary); margin-right: 5px;", "Git" } "Amicus"
                }
                div { class: "title-section-center", 
                    div { class: "repo-status-box",
                        span { class: "repo-name", "{repo_name}" }
                        span { class: "divider", "|" }
                        span { class: "branch-icon", "⎇" }
                        span { class: "branch-name", "{current_branch}" }
                    }
                }
                div { class: "title-section-right",
                    if *status_msg.read() != "ready" {
                        div { class: "status-box", "{i18n.translate(&status_msg.read().to_uppercase())}" }
                    }
                    div { class: "window-controls",
                        div { class: "control-btn", 
                            onmousedown: |e| e.stop_propagation(),
                            onclick: |e| { 
                                e.stop_propagation();
                                dioxus::desktop::window().set_minimized(true); 
                            }, 
                            "_" 
                        }
                        div { class: "control-btn", 
                            onmousedown: |e| e.stop_propagation(),
                            onclick: |e| { 
                                e.stop_propagation();
                                let w = dioxus::desktop::window(); 
                                if w.is_maximized() { w.set_maximized(false); } else { w.set_maximized(true); } 
                            }, 
                            "☐" 
                        }
                        div { class: "control-btn close", 
                            onmousedown: |e| e.stop_propagation(),
                            onclick: |e| { 
                                e.stop_propagation();
                                std::thread::spawn::<_, ()>(|| std::process::exit(0)); 
                            }, 
                            "✕" 
                        }
                    }
                }
            }
            
            div { class: "menu-bar", style: "background: var(--bg-header); border-bottom: 1px solid var(--border-color); padding: 2px 5px; height: 32px; display: flex; align-items: center;",
                div {
                    style: "position: relative;",
                    div {
                        class: "menu-item", 
                        onmousedown: move |e| e.stop_propagation(),
                        onclick: move |e| { e.stop_propagation(); let current = active_menu.read().clone(); let new_val = if current == Some("file".to_string()) { None } else { Some("file".to_string()) }; active_menu.set(new_val); }, 
                        "{i18n.translate(\"m-file\")}" 
                    }
                    if *active_menu.read() == Some("file".to_string()) {
                        div { class: "menu-dropdown", onmousedown: move |e| e.stop_propagation(),
                            div { class: "dropdown-item", onclick: move |_| { if let Some(p) = rfd::FileDialog::new().pick_folder() { let new_path = p.display().to_string(); repo_path.set(new_path.clone()); let _ = save_credentials(&git_user.read(), &git_token.read(), &new_path); active_menu.set(None); let n = *refresh_trigger.read() + 1; refresh_trigger.set(n); } }, "{i18n.translate(\"mi-open\")}" }
                            div { class: "dropdown-item", onclick: move |_| { if let Some(p) = rfd::FileDialog::new().pick_folder() { let new_path = p.display().to_string(); match GitHandler::init(&new_path) { Ok(_) => { status_msg.set("Init success".to_string()); repo_path.set(new_path.clone()); let _ = save_credentials(&git_user.read(), &git_token.read(), &new_path); }, Err(e) => status_msg.set(format!("Init Error: {}", e)), } active_menu.set(None); } }, "{i18n.translate(\"mi-init\")}" }
                            div { class: "dropdown-item", onclick: move |_| { show_clone_modal.set(true); active_menu.set(None); }, "{i18n.translate(\"mi-clone\")}" }
                            div { class: "separator" }
                            div { class: "dropdown-item", onclick: move |_| { show_settings_modal.set(true); active_menu.set(None); }, "{i18n.translate(\"mi-settings\")}" }
                            div { class: "separator" }
                            div { class: "dropdown-item", onclick: move |_| { std::thread::spawn::<_, ()>(|| { std::process::exit(0); }); }, "{i18n.translate(\"mi-exit\")}" }
                        } 
                    }
                }
                div { style: "position: relative;",
                    div { class: "menu-item", onmousedown: move |e| e.stop_propagation(), onclick: move |e| { e.stop_propagation(); let current = active_menu.read().clone(); let new_val = if current == Some("view".to_string()) { None } else { Some("view".to_string()) }; active_menu.set(new_val); }, "{i18n.translate(\"m-view\")}" }
                    if *active_menu.read() == Some("view".to_string()) {
                        div { class: "menu-dropdown", onmousedown: move |e| e.stop_propagation(),
                            div { class: "dropdown-item", onclick: move |_| { let v = *view_all_commits.read(); view_all_commits.set(!v); let n = *refresh_trigger.read() + 1; refresh_trigger.set(n); active_menu.set(None); }, "{i18n.translate(\"mi-view-all\")}" }
                        }
                    }
                }
                div { style: "position: relative;",
                    div { class: "menu-item", onmousedown: move |e| e.stop_propagation(), onclick: move |e| { e.stop_propagation(); let current = active_menu.read().clone(); let new_val = if current == Some("repo".to_string()) { None } else { Some("repo".to_string()) }; active_menu.set(new_val); }, "{i18n.translate(\"m-repo\")}" }
                    if *active_menu.read() == Some("repo".to_string()) {
                        div { class: "menu-dropdown", onmousedown: move |e| e.stop_propagation(),
                            div { class: "dropdown-item", onclick: move |_| { let n = *refresh_trigger.read() + 1; refresh_trigger.set(n); active_menu.set(None); }, span { "{i18n.translate(\"mi-refresh\")}" }, span { style: "color: #888;", "F5" } }
                            div { class: "separator" }
                            div { class: "dropdown-item", onclick: move |_| { active_menu.set(None); spawn(async move { let p = repo_path.read().clone(); let u = git_user.read().clone(); let t = git_token.read().clone(); status_msg.set("Fetching...".to_string()); match GitHandler::fetch(&p, &u, &t) { Ok(_) => status_msg.set("Fetch successful".to_string()), Err(e) => status_msg.set(format!("Fetch Error: {}", e)), } let n = *refresh_trigger.read() + 1; refresh_trigger.set(n); }); }, "{i18n.translate(\"mi-fetch\")}" }
                            div { class: "dropdown-item", onclick: move |_| { active_menu.set(None); spawn(async move { let p = repo_path.read().clone(); let u = git_user.read().clone(); let t = git_token.read().clone(); status_msg.set("Pulling...".to_string()); match GitHandler::pull(&p, &u, &t) { Ok(_) => status_msg.set("Pull successful".to_string()), Err(e) => status_msg.set(format!("Pull Error: {}", e)), } let n = *refresh_trigger.read() + 1; refresh_trigger.set(n); }); }, "{i18n.translate(\"mi-pull\")}" }
                            div { class: "dropdown-item", onclick: move |_| { active_menu.set(None); spawn(async move { let p = repo_path.read().clone(); let u = git_user.read().clone(); let t = git_token.read().clone(); status_msg.set("Pushing...".to_string()); match GitHandler::push(&p, &u, &t, false) { Ok(_) => status_msg.set("Push successful".to_string()), Err(e) => status_msg.set(format!("Push Error: {}", e)), } let n = *refresh_trigger.read() + 1; refresh_trigger.set(n); }); }, "{i18n.translate(\"mi-push\")}" }
                            div { class: "separator" }
                            div { class: "dropdown-item", onclick: move |_| { show_branch_modal.set(true); active_menu.set(None); }, "{i18n.translate(\"mi-new-branch\")}" }
                            div { class: "dropdown-item", onclick: move |_| { show_tag_modal.set(true); active_menu.set(None); }, "{i18n.translate(\"mi-new-tag\")}" }
                            div { class: "dropdown-item", onclick: move |_| { if let Some(f) = rfd::FileDialog::new().pick_file() { let p = repo_path.read().clone(); match GitHandler::apply_patch(&p, &f.display().to_string()) { Ok(_) => { status_msg.set("Patch applied".to_string()); let n = *refresh_trigger.read() + 1; refresh_trigger.set(n); }, Err(e) => status_msg.set(format!("Patch Error: {}", e)), } } active_menu.set(None); }, "{i18n.translate(\"mi-apply-patch\")}" }
                            div { class: "separator" }
                            div { class: "dropdown-item", onclick: move |_| { let _ = Command::new("explorer").arg(&*repo_path.read()).spawn(); active_menu.set(None); }, "{i18n.translate(\"mi-explorer\")}" }
                            div { class: "dropdown-item", onclick: move |_| { if cfg!(target_os = "windows") { let _ = Command::new("cmd").arg("/C").arg("start").current_dir(&*repo_path.read()).spawn(); } active_menu.set(None); }, "{i18n.translate(\"mi-console\")}" } 
                        }
                    }
                }
                div { style: "position: relative;",
                    div { class: "menu-item", onmousedown: move |e| e.stop_propagation(), onclick: move |e| { e.stop_propagation(); let current = active_menu.read().clone(); let new_val = if current == Some("window".to_string()) { None } else { Some("window".to_string()) }; active_menu.set(new_val); }, "{i18n.translate(\"m-window\")}" }
                    if *active_menu.read() == Some("window".to_string()) {
                        div { class: "menu-dropdown", onmousedown: move |e| e.stop_propagation(),
                            div { class: "dropdown-item", onclick: move |_| { let z = *zoom_level.read() + 0.1; zoom_level.set(z); }, span { "{i18n.translate(\"mi-zoom-in\")}" }, span { style: "color: #888;", "Ctrl++" } }
                            div { class: "dropdown-item", onclick: move |_| { let z = *zoom_level.read() - 0.1; zoom_level.set(z); }, span { "{i18n.translate(\"mi-zoom-out\")}" }, span { style: "color: #888;", "Ctrl+-" } }
                            div { class: "separator" }
                            div { class: "dropdown-item", "{i18n.translate(\"mi-theme\")}" }
                        }
                    }
                } 
                div { class: "menu-item", onmousedown: move |e| e.stop_propagation(), "{i18n.translate(\"m-help\")}" }
            }

            div { class: "toolbar", style: "height: 44px; background: var(--bg-base); border-bottom: 1px solid var(--border-color); display: flex; align-items: center; padding: 0 15px; gap: 8px; flex-shrink: 0;",
                div { class: "toolbar-btn", onmousedown: move |e| e.stop_propagation(), onclick: move |_| { if let Some(p) = rfd::FileDialog::new().pick_folder() { let new_path = p.display().to_string(); repo_path.set(new_path.clone()); let _ = save_credentials(&git_user.read(), &git_token.read(), &new_path); let n = *refresh_trigger.read() + 1; refresh_trigger.set(n); } }, span { style: "font-size: 1.2em;", "📂" }, "{i18n.translate(\"btn-open-repo\")}" }
                div { class: "toolbar-btn", onmousedown: move |e| e.stop_propagation(), onclick: move |_| { spawn(async move { let p = repo_path.read().clone(); let u = git_user.read().clone(); let t = git_token.read().clone(); status_msg.set("Pulling...".to_string()); match GitHandler::pull(&p, &u, &t) { Ok(_) => status_msg.set("Pull successful".to_string()), Err(e) => status_msg.set(format!("Pull Error: {}", e)), } let n = *refresh_trigger.read() + 1; refresh_trigger.set(n); }); }, span { style: "font-size: 1.2em;", "⬇" }, "{i18n.translate(\"btn-pull\")}" }
                
                div { style: "position: relative;",
                    div { class: "toolbar-btn", onmousedown: move |e| e.stop_propagation(), onclick: move |e| { e.stop_propagation(); let current = *show_push_menu.read(); show_push_menu.set(!current); }, span { style: "font-size: 1.2em;", "⬆" }, "{i18n.translate(\"btn-push\")}", " ▾" }
                    if *show_push_menu.read() {
                        div { class: "menu-dropdown", onmousedown: move |e| e.stop_propagation(),
                            div { class: "dropdown-item", onclick: move |_| { show_push_menu.set(false); spawn(async move { let p = repo_path.read().clone(); let u = git_user.read().clone(); let t = git_token.read().clone(); status_msg.set("Pushing...".to_string()); match GitHandler::push(&p, &u, &t, false) { Ok(_) => status_msg.set("Push successful".to_string()), Err(e) => status_msg.set(format!("Push Error: {}", e)), } let n = *refresh_trigger.read() + 1; refresh_trigger.set(n); }); }, "{i18n.translate(\"btn-push\")}" }
                            div { class: "dropdown-item", style: "color: var(--accent-secondary);", onclick: move |_| { show_push_menu.set(false); spawn(async move { let p = repo_path.read().clone(); let u = git_user.read().clone(); let t = git_token.read().clone(); status_msg.set("Force Pushing...".to_string()); match GitHandler::push(&p, &u, &t, true) { Ok(_) => status_msg.set("Force Push successful".to_string()), Err(e) => status_msg.set(format!("Force Push Error: {}", e)), } let n = *refresh_trigger.read() + 1; refresh_trigger.set(n); }); }, "{i18n.translate(\"btn-force-push\")}" }
                        }
                    }
                }
                div { class: "toolbar-btn", onmousedown: move |e| e.stop_propagation(), onclick: move |_| { spawn(async move { let p = repo_path.read().clone(); match GitHandler::stash_save(&p) { Ok(_) => { status_msg.set("Stash saved".to_string()); let n = *refresh_trigger.read()+1; refresh_trigger.set(n); }, Err(e) => status_msg.set(format!("Stash Error: {}", e)), } }); }, span { style: "font-size: 1.2em;", "📦" }, "{i18n.translate(\"btn-stash-save\")}" }
                div { class: "toolbar-btn", onmousedown: move |e| e.stop_propagation(), onclick: move |_| { spawn(async move { let p = repo_path.read().clone(); match GitHandler::stash_pop(&p) { Ok(_) => { status_msg.set("Stash popped".to_string()); let n = *refresh_trigger.read()+1; refresh_trigger.set(n); }, Err(e) => status_msg.set(format!("Pop Error: {}", e)), } }); }, span { style: "font-size: 1.2em;", "📤" }, "{i18n.translate(\"btn-stash-pop\")}" }
            }

            div {
                style: "display: flex; flex: 1; overflow: hidden; min-height: 0;",
                
                div {
                    style: "width: {sidebar_width}px; background: var(--bg-sidebar); border-right: 1px solid var(--border-color); display: flex; flex-direction: column; overflow-y: auto; flex-shrink: 0;",
                    
                    div { class: "workspace-header", "WORKSPACE" } 
                    div { class: if *view_mode.read() == "local" { "nav-item active" } else { "nav-item" },
                        onclick: move |_| { view_mode.set("local".to_string()); selected_commit.set(None); selected_file.set(None); },
                        span { "{i18n.translate(\"local\")}" }
                        span { class: "badge", style: "margin-left: auto;", "{staged_files.len() + unstaged_files.len()}" }
                    }
                    
                    div { class: "workspace-header", "BRANCHES" } 
                    div { style: "padding: 0 15px 5px 15px; display: flex; gap: 5px;", 
                        input { class: "input-modern", style: "flex: 1; padding: 8px 10px;", placeholder: "New branch...", value: "{new_branch_name}", oninput: move |evt| new_branch_name.set(evt.value()) }
                        button { class: "btn-icon", onclick: move |_| { let path = repo_path.read().clone(); let name = new_branch_name.read().clone(); if !name.is_empty() { match GitHandler::create_branch(&path, &name) { Ok(_) => { status_msg.set("Branch created".to_string()); new_branch_name.set("".to_string()); let n = *refresh_trigger.read()+1; refresh_trigger.set(n); }, Err(e) => status_msg.set(format!("Error: {}", e)), } } }, "+" }
                    }
                    ul { style: "list-style: none; padding: 0; margin: 0;",
                        for branch in branches {
                            {
                                let is_head = branch == current_branch;
                                let b_name = branch.clone();
                                let b_ctx = branch.clone();
                                let item_class = if is_head { "nav-item active" } else { "nav-item" };
                                let icon = if is_head { "●" } else { "○" };
                                rsx! { 
                                    li { class: "{item_class}",
                                        onclick: move |_| { let p = repo_path.read().clone(); match GitHandler::checkout_branch(&p, &b_name) { Ok(_) => { let n = *refresh_trigger.read()+1; refresh_trigger.set(n); }, Err(e) => status_msg.set(format!("Checkout Error: {}", e)), } },
                                        oncontextmenu: move |evt| { evt.stop_propagation(); context_menu_pos.set(Some((evt.page_coordinates().x, evt.page_coordinates().y, "branch".to_string(), b_ctx.clone()))); },
                                        prevent_default: "oncontextmenu",
                                        span { style: "margin-right: 5px;", "{icon}" }
                                        "{branch}" 
                                    } 
                                }
                            }
                        }
                    }
                    div { class: "workspace-header", "TAGS" } 
                    ul { style: "list-style: none; padding: 0; margin: 0;", for tag in tags { { let t_name = tag.clone(); rsx! { li { class: "nav-item", onclick: move |_| { let p = repo_path.read().clone(); match GitHandler::checkout_branch(&p, &t_name) { Ok(_) => { let n = *refresh_trigger.read()+1; refresh_trigger.set(n); }, Err(e) => status_msg.set(format!("Checkout Error: {}", e)), } }, span { "🏷" } "{tag}" } } } } }
                    div { class: "workspace-header", "REMOTES" }
                    ul { style: "list-style: none; padding: 0; margin: 0;", for rb in remote_branches { { let r_name = rb.clone(); rsx! { li { class: "nav-item", onclick: move |_| { let p = repo_path.read().clone(); match GitHandler::checkout_branch(&p, &r_name) { Ok(_) => { let n = *refresh_trigger.read()+1; refresh_trigger.set(n); }, Err(e) => status_msg.set(format!("Checkout Error: {}", e)), } }, span { "☁" } "{rb}" } } } } }
                }
                
                div { class: "resizer", onmousedown: move |_| dragging_sidebar.set(true) }

                div { 
                    style: "flex: 1; display: flex; flex-direction: column; background: var(--bg-base); border-right: 1px solid var(--border-color); min-width: 300px; overflow: hidden;",
                    div {
                        style: "padding: 8px; border-bottom: 1px solid var(--border-color); display: flex; gap: 10px; background: var(--bg-header); flex-shrink: 0;",
                        input { class: "input-modern", style: "flex: 1;", placeholder: "Search commits...", value: "{commit_search}", oninput: move |evt| commit_search.set(evt.value()) }
                    }
                    div {
                        style: "flex: 1; overflow-y: auto;",
                        table { 
                            thead {
                                tr {
                                    style: "background: var(--bg-sidebar); text-align: left; color: var(--text-sub); position: sticky; top: 0; z-index: 10;",
                                    th { style: "width: 120px;", "Graph" }
                                    th { "Description" }
                                    th { style: "width: 140px;", "Date" }
                                    th { style: "width: 120px;", "Author" }
                                }
                            }
                            tbody {
                                for (sha, summary, author, time, parents) in commits.iter() {
                                    {
                                        if let Some(node) = graph_map.get(sha) {
                                            let is_sel = Some(sha.clone()) == *selected_commit.read();
                                            let bg_val = if is_sel { "var(--table-hover)" } else { "transparent" };
                                            let sha_click = sha.clone();
                                            let sha_ctx = sha.clone();
                                            let summary_clone = summary.clone();
                                            let parents_clone = parents.clone();
                                            rsx! {
                                                tr {
                                                    style: "background: {bg_val}; height: 28px; cursor: pointer;",
                                                    onclick: move |_| {
                                                        selected_commit.set(Some(sha_click.clone()));
                                                        view_mode.set("history".to_string());
                                                        selected_file.set(None);
                                                    },
                                                    oncontextmenu: move |evt| {
                                                        evt.stop_propagation();
                                                        if parents_clone.len() <= 1 {
                                                            context_menu_pos.set(Some((evt.page_coordinates().x, evt.page_coordinates().y, "commit".to_string(), sha_ctx.clone())));
                                                            reword_input.set(summary_clone.clone());
                                                        }
                                                    },
                                                    prevent_default: "oncontextmenu",
                                                    td { class: "commit-graph-cell-svg",
                                                        svg {
                                                            height: "32px",
                                                            width: "140px",
                                                            for (idx, path_d) in node.paths.iter().enumerate() {
                                                                path { 
                                                                    d: "{path_d}", 
                                                                    stroke: "{GRAPH_COLORS[*node.path_colors.get(idx).unwrap_or(&node.color_index)]}", 
                                                                    "stroke-width": "4", 
                                                                    fill: "none",
                                                                    "stroke-linecap": "round",
                                                                    "stroke-linejoin": "round"
                                                                }
                                                            }
                                                            circle {
                                                                cx: "{node.cx}",
                                                                cy: "{node.cy}",
                                                                r: "{node.r}",
                                                                fill: "var(--bg-base)",
                                                                stroke: "{GRAPH_COLORS[node.color_index]}",
                                                                "stroke-width": "3"
                                                            }
                                                        }
                                                    }
                                                    td { "{summary}" }
                                                    td { "{time}" }
                                                    td { "{author}" }
                                                }
                                            }
                                        } else {
                                            rsx! {}
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                
                div { class: "resizer", onmousedown: move |_| dragging_right.set(true) }

                div {
                    style: "width: {right_panel_width}px; background: var(--bg-base); display: flex; flex-direction: column; overflow: hidden;",
                    
                    div {
                        style: "padding: 0 15px; height: 36px; border-bottom: 1px solid var(--border-color); display: flex; align-items: center; gap: 25px; font-size: 0.85em; background: var(--bg-header); flex-shrink: 0;",
                        
                        span { 
                            style: "font-weight: 600; cursor: pointer; height: 100%; display: flex; align-items: center; border-bottom: 2px solid if *right_panel_tab.read() == \"commit\" { \"var(--accent-primary)\" } else { \"none\" }; color: if *right_panel_tab.read() == \"commit\" { \"var(--accent-primary)\" } else { \"var(--text-sub)\" };",
                            onclick: move |_| right_panel_tab.set("commit".to_string()),
                            "{i18n.translate(\"tab-commit\")}" 
                        }
                        span { 
                            style: "color: var(--text-sub); font-weight: 600; border-bottom: if *right_panel_tab.read() == \"changes\" { \"2px solid var(--accent-primary)\" } else { \"none\" }; height: 100%; display: flex; align-items: center; cursor: pointer;",
                            onclick: move |_| right_panel_tab.set("changes".to_string()),
                            "{i18n.translate(\"tab-changes\")}" 
                        }
                    }
                    
                    if *view_mode.read() == "local" {
                        if *right_panel_tab.read() == "commit" {
                            div {
                                style: "flex: 1; display: flex; flex-direction: column; overflow: hidden;",
                                
                                div {
                                    style: "padding: 15px; border-bottom: 1px solid var(--border-color); background: var(--bg-surface); display: flex; flex-direction: column; gap: 10px; flex-shrink: 0;",
                                    textarea {
                                        class: "input-modern",
                                        style: "width: 100%; padding: 10px; border: 1px solid var(--border-color); border-radius: 4px; height: 70px; font-size: 0.9em; resize: none; font-family: 'Inter', sans-serif; background: var(--bg-surface); color: white; outline: none;",
                                        placeholder: "Commit message...",
                                        value: "{commit_msg}",
                                        oninput: move |evt| commit_msg.set(evt.value())
                                    }
                                    div {
                                        style: "display: flex; justify-content: flex-end; gap: 10px;",
                                        button {
                                            class: "btn-primary",
                                            onclick: move |_| {
                                                let msg = commit_msg.read().clone();
                                                let path = repo_path.read().clone();
                                                if !msg.is_empty() {
                                                    match GitHandler::create_commit(&path, &msg) {
                                                        Ok(_) => { 
                                                            status_msg.set("Committed".to_string()); 
                                                            commit_msg.set("".to_string());
                                                            let next = *refresh_trigger.read() + 1; refresh_trigger.set(next);
                                                        },
                                                        Err(e) => status_msg.set(format!("Commit Error: {}", e)),
                                                    }
                                                }
                                            },
                                            "Commit"
                                        }
                                    }
                                }

                                div { style: "padding: 8px 15px; background: rgba(137, 180, 250, 0.1); font-weight: 600; font-size: 0.75em; border-bottom: 1px solid var(--border-color); color: var(--accent-primary); flex-shrink: 0;", "{i18n.translate(\"staged-changes\")}" }
                                ul { style: "list-style: none; padding: 0; margin: 0; flex: 0.4; overflow-y: auto; background: var(--bg-base); min-height: 0; border-bottom: 1px solid var(--border-color);",
                                    for file in staged_files {
                                        {
                                            let is_sel = Some(file.clone()) == *selected_file.read();
                                            let bg_val = if is_sel { "var(--bg-surface)" } else { "transparent" };
                                            let f_sel = file.clone();
                                            let f_unstage = file.clone();
                                            let f_ctx = file.clone();
                                            rsx! {
                                                li { 
                                                    style: "padding: 5px 15px; font-size: 0.85em; cursor: pointer; background: {bg_val}; border-bottom: 1px solid var(--border-color); display: flex; align-items: center; gap: 8px; color: var(--text-main);",
                                                    onclick: move |_| selected_file.set(Some(f_sel.clone())),
                                                    oncontextmenu: move |evt| {
                                                        evt.stop_propagation();
                                                        context_menu_pos.set(Some((evt.page_coordinates().x, evt.page_coordinates().y, "file_staged".to_string(), f_ctx.clone())));
                                                    },
                                                    prevent_default: "oncontextmenu",
                                                    input {
                                                        r#type: "checkbox",
                                                        checked: "true",
                                                        onchange: move |evt| {
                                                            evt.stop_propagation();
                                                            let path = repo_path.read().clone();
                                                            let _ = GitHandler::unstage_files(&path, vec![f_unstage.clone()]);
                                                            let next = *refresh_trigger.read() + 1; refresh_trigger.set(next);
                                                        }
                                                    }
                                                    span { "{file}" }
                                                }
                                            }
                                        }
                                    }
                                }

                                div { style: "padding: 8px 15px; background: rgba(243, 139, 168, 0.1); font-weight: 600; font-size: 0.75em; border-bottom: 1px solid var(--border-color); color: var(--accent-secondary); flex-shrink: 0;", "{i18n.translate(\"unstaged-changes\")}" }
                                ul { style: "list-style: none; padding: 0; margin: 0; flex: 0.4; overflow-y: auto; background: var(--bg-base); min-height: 0;",
                                    for file in unstaged_files {
                                        {
                                            let is_sel = Some(file.clone()) == *selected_file.read();
                                            let bg_val = if is_sel { "var(--bg-surface)" } else { "transparent" };
                                            let f_sel = file.clone();
                                            let f_stage = file.clone();
                                            let f_ctx = file.clone();
                                            rsx! {
                                                li { 
                                                    style: "padding: 5px 15px; font-size: 0.85em; cursor: pointer; background: {bg_val}; border-bottom: 1px solid var(--border-color); display: flex; align-items: center; gap: 8px; color: var(--text-main);",
                                                    onclick: move |_| selected_file.set(Some(f_sel.clone())),
                                                    oncontextmenu: move |evt| {
                                                        evt.stop_propagation();
                                                        context_menu_pos.set(Some((evt.page_coordinates().x, evt.page_coordinates().y, "file_unstaged".to_string(), f_ctx.clone())));
                                                    },
                                                    prevent_default: "oncontextmenu",
                                                    input {
                                                        r#type: "checkbox",
                                                        checked: "false",
                                                        onchange: move |evt| {
                                                            evt.stop_propagation();
                                                            let path = repo_path.read().clone();
                                                            let _ = GitHandler::stage_files(&path, vec![f_stage.clone()]);
                                                            let next = *refresh_trigger.read() + 1; refresh_trigger.set(next);
                                                        }
                                                    }
                                                    span { "{file}" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        } else {
                            div { style: "flex: 1; background: #1e1e2e; color: #cdd6f4; overflow: auto; font-family: 'JetBrains Mono', monospace; font-size: 0.85em; padding: 10px; min-height: 0;",
                                for line in diff_content.lines() {
                                    {
                                        let line_bg = if line.starts_with('+') { "rgba(166, 227, 161, 0.2)" } else if line.starts_with('-') { "rgba(243, 139, 168, 0.2)" } else { "transparent" };
                                        rsx! { div { style: "background: {line_bg}; line-height: 1.4; white-space: pre;", "{line}" } }
                                    }
                                }
                            }
                        }
                    } else {
                        if let Some((author, committer, msg, sha, parents)) = commit_details {
                            div { style: "flex: 1; display: flex; flex-direction: column; overflow: hidden;",
                                div { style: "padding: 20px; border-bottom: 1px solid var(--border-color); background: var(--bg-surface); flex-shrink: 0;",
                                    div { style: "display: flex; gap: 15px; margin-bottom: 15px;",
                                        div { style: "width: 48px; height: 48px; background: var(--accent-primary); border-radius: 8px; display: flex; align-items: center; justify-content: center; font-weight: bold; font-size: 1.4em; color: var(--bg-base);", "{author[..1].to_uppercase()}" }
                                        div {
                                            div { style: "font-weight: 700; font-size: 1.1em; color: #fff;", "{author}" }
                                            div { style: "font-size: 0.85em; color: var(--text-sub); margin-top: 2px;", "Committed by {committer}" }
                                            div { style: "font-size: 0.8em; color: var(--accent-primary); margin-top: 6px; font-family: 'JetBrains Mono', monospace; background: rgba(0,0,0,0.2); padding: 2px 5px; border-radius: 3px; display: inline-block;", "{sha}" }
                                            div { style: "font-size: 0.8em; color: var(--text-sub); margin-top: 2px;", "Parents: {parents}" }
                                        }
                                    }
                                    div { style: "white-space: pre-wrap; font-size: 0.95em; line-height: 1.5; color: var(--text-main); background: rgba(0,0,0,0.2); padding: 12px; border-radius: 6px; border: 1px solid var(--border-color);", "{msg}" }
                                }
                                div { style: "padding: 8px 15px; background: var(--bg-header); font-weight: 600; font-size: 0.75em; border-bottom: 1px solid var(--border-color); color: var(--text-sub); flex-shrink: 0;", "CHANGED FILES" }
                                ul { style: "list-style: none; padding: 0; margin: 0; flex: 0.3; overflow-y: auto; background: var(--bg-base); min-height: 0;",
                                    for file in commit_files {
                                        {
                                            let is_sel = Some(file.clone()) == *selected_file.read();
                                            let bg_val = if is_sel { "var(--bg-surface)" } else { "transparent" };
                                            let f_sel = file.clone();
                                            rsx! {
                                                li { 
                                                    style: "padding: 5px 15px; font-size: 0.85em; cursor: pointer; background: {bg_val}; border-bottom: 1px solid var(--border-color); color: var(--text-main);",
                                                    onclick: move |_| selected_file.set(Some(f_sel.clone())),
                                                    "{file}"
                                                }
                                            }
                                        }
                                    }
                                }

                                div { style: "flex: 0.7; background: #1e1e2e; color: #cdd6f4; overflow: auto; font-family: 'JetBrains Mono', monospace; font-size: 0.8em; border-top: 1px solid var(--border-color); min-height: 0; padding: 10px;",
                                    for line in diff_content.lines() {
                                        {
                                            let line_bg = if line.starts_with('+') { "rgba(166, 227, 161, 0.2)" } else if line.starts_with('-') { "rgba(243, 139, 168, 0.2)" } else { "transparent" };
                                            rsx! { div { style: "background: {line_bg}; line-height: 1.4; white-space: pre;", "{line}" } }
                                        }
                                    }
                                }
                            }
                        } else {
                            div { style: "padding: 40px; color: var(--text-sub); text-align: center;", "Select a commit to view details" }
                        }
                    }
                }
            }

            if let Some((x, y, ref m_type, ref target)) = *context_menu_pos.read() {
                {
                    let t1 = target.clone();
                    let t2 = target.clone();
                    let t3 = target.clone();
                    let t4_explorer = target.clone();
                    let t4_discard = target.clone();
                    let t5_checkout = target.clone();
                    let t6_reset = target.clone();
                    let t8_ignore = target.clone();
                    let t9_checkout_b = target.clone();
                    let t10_del_b = target.clone();
                    let t_patch = target.clone();
                    let t_info = target.clone();
                    let t_file_staged = target.clone();
                    let t_file_unstaged = target.clone();

                    let p_cherry = repo_path.read().clone();
                    let p_revert = repo_path.read().clone();
                    let p_discard = repo_path.read().clone();
                    let p_checkout = repo_path.read().clone();
                    let p_reset = repo_path.read().clone();
                    let p_ignore = repo_path.read().clone();
                    let p_checkout_b = repo_path.read().clone();
                    let p_del_b = repo_path.read().clone();
                    let p_squash = repo_path.read().clone();
                    let p_patch = repo_path.read().clone();
                    let p_info = repo_path.read().clone();
                    let p_stage = repo_path.read().clone();
                    let p_unstage = repo_path.read().clone();
                    
                    let menu_type = m_type.clone();

                    rsx! {
                        div { class: "context-menu",
                            style: "left: {x}px; top: {y}px;",
                            onmousedown: move |e| e.stop_propagation(), 
                            
                            if menu_type == "commit" {
                                div {
                                    div { style: "padding: 4px 12px; font-weight: bold; color: var(--text-sub); font-size: 0.8em;", "QUICK ACTIONS (HEAD)" }
                                    div { class: "dropdown-item", 
                                        onclick: move |_| { 
                                            show_reword_modal.set(true);
                                            context_menu_pos.set(None); 
                                        }, 
                                        "{i18n.translate(\"menu-reword\")}" 
                                    }
                                    div { class: "dropdown-item", 
                                        onclick: move |_| { 
                                            show_author_modal.set(true);
                                            context_menu_pos.set(None); 
                                        }, 
                                        "{i18n.translate(\"menu-author\")}" 
                                    }
                                    div { class: "dropdown-item", 
                                        onclick: move |_| { 
                                            let _ = GitHandler::squash_parent(&p_squash); 
                                            context_menu_pos.set(None); 
                                            let next = *refresh_trigger.read() + 1; refresh_trigger.set(next); 
                                        }, 
                                        "{i18n.translate(\"menu-squash\")}" 
                                    }
                                    div { class: "separator" }
                                    div { class: "dropdown-item", onclick: move |_| { let _ = GitHandler::checkout_commit(&p_checkout, &t5_checkout); context_menu_pos.set(None); let next = *refresh_trigger.read() + 1; refresh_trigger.set(next); }, "{i18n.translate(\"menu-checkout\")}" }
                                    div { class: "dropdown-item", onclick: move |_| { let _ = GitHandler::cherry_pick(&p_cherry, &t1); context_menu_pos.set(None); }, "{i18n.translate(\"menu-cherry\")}" }
                                    div { class: "dropdown-item", onclick: move |_| { let _ = GitHandler::revert_commit(&p_revert, &t2); context_menu_pos.set(None); }, "{i18n.translate(\"menu-revert\")}" }
                                    div { class: "dropdown-item", onclick: move |_| { let _ = GitHandler::save_patch(&p_patch, &t_patch); context_menu_pos.set(None); }, "{i18n.translate(\"menu-patch\")}" }
                                    div { class: "dropdown-item", style: "color: var(--accent-secondary);", onclick: move |_| { let _ = GitHandler::reset_hard(&p_reset, &t6_reset); context_menu_pos.set(None); let next = *refresh_trigger.read() + 1; refresh_trigger.set(next); }, "{i18n.translate(\"menu-reset\")}" }
                                    div { class: "separator" }
                                    div { class: "dropdown-item", onclick: move |_| { let _ = Command::new("powershell").arg("-Command").arg(format!("Set-Clipboard -Value '{}'", t3)).spawn(); context_menu_pos.set(None); }, "{i18n.translate(\"menu-copy\")}" }
                                    div { class: "dropdown-item", 
                                        onclick: move |_| { 
                                            if let Ok((a, c, m, s, _)) = GitHandler::get_commit_details(&p_info, &t_info) {
                                                let info = format!("Commit: {}\nAuthor: {}\nDate: {}\nMessage: {}", s, a, c, m);
                                                let _ = Command::new("powershell").arg("-Command").arg(format!("Set-Clipboard -Value '{}'", info)).spawn();
                                            }
                                            context_menu_pos.set(None); 
                                        }, 
                                        "{i18n.translate(\"menu-copy-info\")}" 
                                    }
                                }
                            } else if menu_type == "file_staged" {
                                div {
                                    div { class: "dropdown-item", onclick: move |_| { let _ = Command::new("explorer").arg("/select,").arg(&t4_explorer).spawn(); context_menu_pos.set(None); }, "{i18n.translate(\"menu-open\")}" }
                                    div { class: "dropdown-item", 
                                        onclick: move |_| { 
                                            let _ = GitHandler::unstage_files(&p_unstage, vec![t_file_staged.clone()]);
                                            context_menu_pos.set(None);
                                            let next = *refresh_trigger.read() + 1; refresh_trigger.set(next);
                                        }, 
                                        "{i18n.translate(\"menu-unstage\")}" 
                                    }
                                }
                            } else if menu_type == "file_unstaged" {
                                div {
                                    div { class: "dropdown-item", onclick: move |_| { let _ = Command::new("explorer").arg("/select,").arg(&t4_explorer).spawn(); context_menu_pos.set(None); }, "{i18n.translate(\"menu-open\")}" }
                                    div { class: "dropdown-item", 
                                        onclick: move |_| { 
                                            let _ = GitHandler::stage_files(&p_stage, vec![t_file_unstaged.clone()]);
                                            context_menu_pos.set(None);
                                            let next = *refresh_trigger.read() + 1; refresh_trigger.set(next);
                                        }, 
                                        "{i18n.translate(\"menu-stage\")}" 
                                    }
                                    div { class: "dropdown-item", onclick: move |_| { let _ = GitHandler::add_to_gitignore(&p_ignore, &t8_ignore); context_menu_pos.set(None); let next = *refresh_trigger.read() + 1; refresh_trigger.set(next); }, "{i18n.translate(\"menu-ignore\")}" }
                                    div { class: "separator" }
                                    div { class: "dropdown-item", style: "color: var(--accent-secondary);", onclick: move |_| { let _ = GitHandler::discard_changes(&p_discard, &t4_discard); context_menu_pos.set(None); let next = *refresh_trigger.read() + 1; refresh_trigger.set(next); }, "{i18n.translate(\"menu-discard\")}" }
                                } 
                            } else if menu_type == "branch" { 
                                div {
                                    div { class: "dropdown-item", onclick: move |_| { let _ = GitHandler::checkout_branch(&p_checkout_b, &t9_checkout_b); context_menu_pos.set(None); let next = *refresh_trigger.read() + 1; refresh_trigger.set(next); }, "{i18n.translate(\"menu-checkout-branch\")}" }
                                    div { class: "separator" }
                                    div { class: "dropdown-item", style: "color: var(--accent-secondary);", onclick: move |_| { let _ = GitHandler::delete_branch(&p_del_b, &t10_del_b); context_menu_pos.set(None); let next = *refresh_trigger.read() + 1; refresh_trigger.set(next); }, "{i18n.translate(\"menu-del-branch\")}" }
                                }
                            }
                        }
                    }
                }
            }
            
            if *show_reword_modal.read() {
                div {
                    style: "position: absolute; top: 0; left: 0; width: 100%; height: 100%; background: rgba(0,0,0,0.5); z-index: 5000; display: flex; align-items: center; justify-content: center;",
                    div {
                        style: "background: var(--bg-surface); padding: 25px; border-radius: 12px; width: 420px; box-shadow: 0 10px 30px rgba(0,0,0,0.5); border: 1px solid var(--border-color); color: var(--text-main);",
                        h3 { style: "margin-top: 0;", "{i18n.translate(\"modal-reword-title\")}" }
                        textarea { 
                            class: "input-modern",
                            style: "width: 100%; height: 80px; margin: 10px 0; padding: 5px;",
                            value: "{reword_input}",
                            oninput: move |e| reword_input.set(e.value())
                        }
                        div {
                            style: "display: flex; justify-content: flex-end; gap: 10px;",
                            button { class: "toolbar-btn", onclick: move |_| show_reword_modal.set(false), "{i18n.translate(\"modal-cancel\")}" }
                            button { 
                                class: "btn-primary",
                                onclick: move |_| {
                                    let p = repo_path.read().clone();
                                    let msg = reword_input.read().clone();
                                    match GitHandler::amend_head(&p, Some(&msg), None) {
                                        Ok(_) => status_msg.set("Reword successful".to_string()),
                                        Err(e) => status_msg.set(format!("Error: {}", e)),
                                    }
                                    show_reword_modal.set(false);
                                    let next = *refresh_trigger.read() + 1; refresh_trigger.set(next);
                                }, 
                                "{i18n.translate(\"modal-save\")}" 
                            }
                        }
                    }
                }
            }

            if *show_author_modal.read() {
                div {
                    style: "position: absolute; top: 0; left: 0; width: 100%; height: 100%; background: rgba(0,0,0,0.5); z-index: 5000; display: flex; align-items: center; justify-content: center;",
                    div {
                        style: "background: var(--bg-surface); padding: 25px; border-radius: 12px; width: 420px; box-shadow: 0 10px 30px rgba(0,0,0,0.5); border: 1px solid var(--border-color); color: var(--text-main);",
                        h3 { style: "margin-top: 0;", "{i18n.translate(\"modal-author-title\")}" }
                        input { 
                            class: "input-modern",
                            style: "width: 100%; margin: 8px 0;", placeholder: "Name",
                            value: "{author_name_input}", oninput: move |e| author_name_input.set(e.value())
                        }
                        input { 
                            class: "input-modern",
                            style: "width: 100%; margin: 8px 0;", placeholder: "Email",
                            value: "{author_email_input}", oninput: move |e| author_email_input.set(e.value())
                        }
                        div {
                            style: "display: flex; justify-content: flex-end; gap: 10px; margin-top: 15px;",
                            button { class: "toolbar-btn", onclick: move |_| show_author_modal.set(false), "{i18n.translate(\"modal-cancel\")}" }
                            button { 
                                class: "btn-primary",
                                onclick: move |_| {
                                    let p = repo_path.read().clone();
                                    let name = author_name_input.read().clone();
                                    let email = author_email_input.read().clone();
                                    if !name.is_empty() && !email.is_empty() {
                                        match GitHandler::amend_head(&p, None, Some((&name, &email))) {
                                            Ok(_) => status_msg.set("Author changed".to_string()),
                                            Err(e) => status_msg.set(format!("Error: {}", e)),
                                        }
                                        show_author_modal.set(false);
                                        let next = *refresh_trigger.read() + 1; refresh_trigger.set(next);
                                    }
                                }, 
                                "{i18n.translate(\"modal-save\")}" 
                            }
                        }
                    }
                }
            }

            if *show_clone_modal.read() {
                div {
                    style: "position: absolute; top: 0; left: 0; width: 100%; height: 100%; background: rgba(0,0,0,0.5); z-index: 5000; display: flex; align-items: center; justify-content: center;",
                    div {
                        style: "background: var(--bg-surface); padding: 25px; border-radius: 12px; width: 420px; box-shadow: 0 10px 30px rgba(0,0,0,0.5); border: 1px solid var(--border-color); color: var(--text-main);",
                        h3 { style: "margin-top: 0;", "{i18n.translate(\"modal-clone-title\")}" }
                        input { 
                            class: "input-modern",
                            style: "width: 100%; margin: 8px 0;", placeholder: "https://github.com/user/repo.git",
                            value: "{clone_url_input}", oninput: move |e| clone_url_input.set(e.value())
                        }
                        div {
                            style: "display: flex; justify-content: flex-end; gap: 10px; margin-top: 15px;",
                            button { class: "toolbar-btn", onclick: move |_| show_clone_modal.set(false), "{i18n.translate(\"modal-cancel\")}" }
                            button { 
                                class: "btn-primary",
                                onclick: move |_| {
                                    if let Some(path) = rfd::FileDialog::new().pick_folder() {
                                        let new_path = path.display().to_string();
                                        let url = clone_url_input.read().clone();
                                        if !url.is_empty() {
                                            match GitHandler::clone(&url, &new_path) {
                                                Ok(_) => {
                                                    status_msg.set("Clone successful".to_string()); 
                                                        let _ = save_credentials(&git_user.read(), &git_token.read(), &new_path);
                                                    repo_path.set(new_path.clone());
                                                },
                                                Err(e) => status_msg.set(format!("Clone Error: {}", e)),
                                            }
                                            show_clone_modal.set(false);
                                            let next = *refresh_trigger.read() + 1; refresh_trigger.set(next);
                                        }
                                    }
                                }, 
                                "{i18n.translate(\"modal-clone\")}" 
                            }
                        }
                    }
                }
            }

            if *show_branch_modal.read() {
                div {
                    style: "position: absolute; top: 0; left: 0; width: 100%; height: 100%; background: rgba(0,0,0,0.5); z-index: 5000; display: flex; align-items: center; justify-content: center;",
                    div {
                        style: "background: var(--bg-surface); padding: 25px; border-radius: 12px; width: 420px; box-shadow: 0 10px 30px rgba(0,0,0,0.5); border: 1px solid var(--border-color); color: var(--text-main);",
                        h3 { style: "margin-top: 0;", "{i18n.translate(\"modal-branch-title\")}" }
                        input { 
                            class: "input-modern",
                            style: "width: 100%; margin: 8px 0;", placeholder: "feature/new-stuff",
                            value: "{modal_branch_name}", oninput: move |e| modal_branch_name.set(e.value())
                        }
                        div {
                            style: "display: flex; justify-content: flex-end; gap: 10px; margin-top: 15px;",
                            button { class: "toolbar-btn", onclick: move |_| show_branch_modal.set(false), "{i18n.translate(\"modal-cancel\")}" }
                            button { 
                                class: "btn-primary",
                                onclick: move |_| {
                                    let p = repo_path.read().clone();
                                    let name = modal_branch_name.read().clone();
                                    if !name.is_empty() {
                                        match GitHandler::create_branch(&p, &name) {
                                            Ok(_) => status_msg.set("Branch created".to_string()),
                                            Err(e) => status_msg.set(format!("Error: {}", e)),
                                        }
                                        show_branch_modal.set(false);
                                        let next = *refresh_trigger.read() + 1; refresh_trigger.set(next);
                                    }
                                }, 
                                "{i18n.translate(\"modal-create\")}" 
                            }
                        }
                    }
                }
            }

            if *show_tag_modal.read() {
                div {
                    style: "position: absolute; top: 0; left: 0; width: 100%; height: 100%; background: rgba(0,0,0,0.5); z-index: 5000; display: flex; align-items: center; justify-content: center;",
                    div {
                        style: "background: var(--bg-surface); padding: 25px; border-radius: 12px; width: 420px; box-shadow: 0 10px 30px rgba(0,0,0,0.5); border: 1px solid var(--border-color); color: var(--text-main);",
                        h3 { style: "margin-top: 0;", "{i18n.translate(\"modal-tag-title\")}" }
                        input { 
                            class: "input-modern",
                            style: "width: 100%; margin: 8px 0;", placeholder: "v1.0.0",
                            value: "{modal_tag_name}", oninput: move |e| modal_tag_name.set(e.value())
                        }
                        div {
                            style: "display: flex; justify-content: flex-end; gap: 10px; margin-top: 15px;",
                            button { class: "toolbar-btn", onclick: move |_| show_tag_modal.set(false), "{i18n.translate(\"modal-cancel\")}" }
                            button { 
                                class: "btn-primary",
                                onclick: move |_| {
                                    let p = repo_path.read().clone();
                                    let name = modal_tag_name.read().clone();
                                    if !name.is_empty() {
                                        match GitHandler::create_tag(&p, &name) {
                                            Ok(_) => status_msg.set("Tag created".to_string()),
                                            Err(e) => status_msg.set(format!("Error: {}", e)),
                                        }
                                        show_tag_modal.set(false);
                                        let next = *refresh_trigger.read() + 1; refresh_trigger.set(next);
                                    }
                                }, 
                                "{i18n.translate(\"modal-create\")}" 
                            }
                        }
                    }
                }
            }

            if *show_settings_modal.read() {
                div {
                    style: "position: absolute; top: 0; left: 0; width: 100%; height: 100%; background: rgba(0,0,0,0.5); z-index: 5000; display: flex; align-items: center; justify-content: center;",
                    div {
                        style: "background: var(--bg-surface); padding: 25px; border-radius: 12px; width: 420px; box-shadow: 0 10px 30px rgba(0,0,0,0.5); border: 1px solid var(--border-color); color: var(--text-main);",
                        h3 { "{i18n.translate(\"modal-settings-title\")}" }
                        div { style: "margin-bottom: 10px; font-weight: bold; font-size: 0.9em;", "Git-Anmeldedaten" }
                        input { 
                            class: "input-modern",
                            style: "width: 100%; margin: 8px 0;", placeholder: "Benutzername",
                            value: "{git_user}", oninput: move |e| git_user.set(e.value())
                        }
                        input { 
                            class: "input-modern",
                            style: "width: 100%; margin: 8px 0;", placeholder: "Token", r#type: "password",
                            value: "{git_token}", oninput: move |e| git_token.set(e.value())
                        }
                        div {
                            style: "display: flex; justify-content: flex-end; gap: 10px; margin-top: 15px;",
                            button { class: "toolbar-btn", onclick: move |_| show_settings_modal.set(false), "Abbrechen" }
                            button {
                                class: "btn-primary",
                                onclick: move |_| {
                                    match save_credentials(&git_user.read(), &git_token.read(), &repo_path.read()) {
                                        Ok(_) => status_msg.set("Anmeldedaten gespeichert!".to_string()),
                                        Err(e) => status_msg.set(format!("Fehler beim Speichern der Anmeldedaten: {}", e)),
                                    }
                                    show_settings_modal.set(false);
                                }, 
                                "Speichern"
                            }
                        }
                    }
                }
            }
        }
    }
}