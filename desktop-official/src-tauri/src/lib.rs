pub(crate) use std::fs;
pub(crate) use std::io::Read;
pub(crate) use std::path::PathBuf;
pub(crate) use std::process::{Command, Stdio};
pub(crate) use std::sync::atomic::AtomicU64;
pub(crate) use std::sync::Mutex;
pub(crate) use std::thread;
pub(crate) use std::time::Duration;

pub(crate) use arboard::Clipboard;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{TrayIconBuilder, TrayIconEvent};
pub(crate) use tauri::{AppHandle, Emitter, Manager, WindowEvent};
pub(crate) use tauri_plugin_autostart::ManagerExt;

mod app_core;
mod commands;
mod shared_types;

pub(crate) use app_core::*;
use commands::*;
pub(crate) use shared_types::*;

fn emit_health(app: &AppHandle) {
    if let Some(state) = app.try_state::<AppState>() {
        let health = state_health(&state);
        let _ = app.emit("health-updated", health);
    }
}

fn emit_runtime_status(app: &AppHandle) {
    if let Some(state) = app.try_state::<AppState>() {
        let runtime = runtime_status_value(&state);
        let _ = app.emit("runtime-updated", runtime);
    }
}

fn emit_tasks_updated(app: &AppHandle) {
    if let Some(state) = app.try_state::<AppState>() {
        let tasks = latest_tasks_value(&state);
        let _ = app.emit("tasks-updated", tasks);
    }
}

fn on_tray_click(app: &AppHandle, id: &str) {
    match id {
        "show" => {
            if let Some(win) = app.get_webview_window("main") {
                let _ = win.show();
                let _ = win.set_focus();
            }
        }
        "start" => {
            if let Some(state) = app.try_state::<AppState>() {
                let _ = start_proxy_service(app.clone(), state);
                emit_health(app);
                emit_runtime_status(app);
                emit_tasks_updated(app);
            }
        }
        "stop" => {
            if let Some(state) = app.try_state::<AppState>() {
                let _ = stop_proxy_service(state);
                emit_health(app);
                emit_runtime_status(app);
                emit_tasks_updated(app);
            }
        }
        "copy-url" => {
            if let Some(state) = app.try_state::<AppState>() {
                let _ = copy_base_url(state);
            }
        }
        "copy-key" => {
            if let Some(state) = app.try_state::<AppState>() {
                let _ = copy_api_key(state);
            }
        }
        "open-config" => {
            if let Ok(dir) = app.path().app_config_dir() {
                #[cfg(target_os = "macos")]
                {
                    let _ = Command::new("open").arg(dir).spawn();
                }
                #[cfg(target_os = "windows")]
                {
                    let _ = Command::new("explorer").arg(dir).spawn();
                }
                #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
                {
                    let _ = Command::new("xdg-open").arg(dir).spawn();
                }
            }
        }
        "quit" => app.exit(0),
        _ => {}
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None::<Vec<&str>>,
        ))
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let settings = load_settings_file(&app.handle()).unwrap_or_default();
            let normalized_settings = if settings.service_executable.trim().is_empty() {
                AppSettings {
                    service_executable: default_service_executable_path(&app.handle()),
                    ..settings
                }
            } else {
                settings
            };
            app.manage(AppState {
                settings: Mutex::new(normalized_settings),
                runtime: Mutex::new(ServiceRuntime {
                    stage: "STOPPED".to_string(),
                    stage_changed_at_unix: now_unix(),
                    ..ServiceRuntime::default()
                }),
                tasks: Mutex::new(Vec::new()),
                task_seq: AtomicU64::new(1),
            });

            let show = MenuItem::with_id(app, "show", "Open", true, None::<&str>)?;
            let start = MenuItem::with_id(app, "start", "Start Service", true, None::<&str>)?;
            let stop = MenuItem::with_id(app, "stop", "Stop Service", true, None::<&str>)?;
            let copy_url = MenuItem::with_id(app, "copy-url", "Copy Base URL", true, None::<&str>)?;
            let copy_key = MenuItem::with_id(app, "copy-key", "Copy API Key", true, None::<&str>)?;
            let open_config = MenuItem::with_id(app, "open-config", "Open Config Folder", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Exit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &start, &stop, &copy_url, &copy_key, &open_config, &quit])?;

            let mut tray_builder = TrayIconBuilder::with_id("main-tray")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| {
                    on_tray_click(app, event.id.as_ref());
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click { button, .. } = event {
                        if button == tauri::tray::MouseButton::Left {
                            on_tray_click(&tray.app_handle(), "show");
                        }
                    }
                });
            if let Some(icon) = app.default_window_icon() {
                tray_builder = tray_builder.icon(icon.clone());
            }
            tray_builder.build(app)?;

            let app_handle = app.handle().clone();
            thread::spawn(move || loop {
                emit_health(&app_handle);
                emit_runtime_status(&app_handle);
                emit_tasks_updated(&app_handle);
                thread::sleep(Duration::from_secs(3));
            });

            on_tray_click(&app.handle(), "start");
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == "main" {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_settings,
            save_settings,
            get_health,
            get_runtime_status,
            get_logs_snapshot,
            clear_logs,
            export_diagnostics_file,
            start_proxy_service,
            stop_proxy_service,
            copy_base_url,
            copy_api_key,
            import_auth_file,
            import_auth_files_batch,
            remove_auth_file,
            patch_auth_file_status,
            start_codex_oauth,
            start_oauth,
            start_iflow_cookie_auth,
            poll_oauth_status,
            poll_multi_oauth_status,
            get_usage_summary,
            get_usage_snapshot,
            get_provider_catalog,
            get_auth_files,
            test_provider_connection,
            fetch_provider_models,
            check_provider_quota,
            chat_test_completion,
            patch_key_policy_models,
            delete_key_policy,
            export_settings_file,
            import_settings_file,
            export_usage_file,
            import_usage_file,
            export_usage_csv_file,
            get_autostart_enabled,
            set_autostart_enabled,
            set_service_port,
            set_logging_to_file_fast,
            set_request_log_fast,
            set_usage_statistics_enabled_fast,
            check_update,
            download_and_open_update_installer,
            get_management_secret_status,
            initialize_management_secret,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
