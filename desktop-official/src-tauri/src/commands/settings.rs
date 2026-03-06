use crate::*;

fn put_bool_to_management(settings: &AppSettings, path: &str, value: bool) -> Result<(), String> {
    let body = serde_json::json!({ "value": value }).to_string();
    let _ = management_request_json(settings, reqwest::Method::PUT, path, Some(body), 8)?;
    Ok(())
}

#[tauri::command]
pub(crate) fn get_settings(state: tauri::State<AppState>) -> AppSettings {
    state
        .settings
        .lock()
        .expect("settings lock poisoned")
        .clone()
}

#[tauri::command]
pub(crate) fn save_settings(
    app: AppHandle,
    state: tauri::State<AppState>,
    settings: AppSettings,
) -> Result<(), String> {
    let task_id = begin_task(&state, "save_settings");
    let mut next_settings = settings.clone();
    let remote_sync_error = apply_third_party_to_remote(&next_settings).err();

    if next_settings.key_policies.is_empty() {
        finish_task(
            &state,
            &task_id,
            "failed",
            "at least one API key is required",
        );
        return Err("at least one API key is required".to_string());
    }

    if let Ok(catalog) = provider_catalog_with_settings(&next_settings) {
        let merged = merge_catalog_models(&catalog);
        if !merged.is_empty() {
            next_settings.available_models = merged;
        }
    }

    {
        let mut current = state
            .settings
            .lock()
            .map_err(|_| "settings lock poisoned".to_string())?;
        *current = next_settings.clone();
    }
    let res = save_settings_file(&app, &next_settings);
    if let Err(e) = &res {
        finish_task(&state, &task_id, "failed", e);
    } else {
        let _ = write_runtime_proxy_config(&app, &next_settings);
        if let Some(e) = remote_sync_error {
            finish_task(
                &state,
                &task_id,
                "partial",
                &format!("settings persisted, remote sync skipped: {}", e),
            );
        } else {
            finish_task(&state, &task_id, "success", "settings persisted");
        }
    }
    res
}

#[tauri::command]
pub(crate) fn export_settings_file(
    state: tauri::State<AppState>,
    file_path: String,
) -> Result<String, String> {
    let settings = state
        .settings
        .lock()
        .map_err(|_| "settings lock poisoned".to_string())?
        .clone();
    let serialized = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    fs::write(file_path.trim(), serialized).map_err(|e| e.to_string())?;
    Ok("ok".to_string())
}

#[tauri::command]
pub(crate) fn import_settings_file(
    app: AppHandle,
    state: tauri::State<AppState>,
    file_path: String,
) -> Result<String, String> {
    let data = fs::read_to_string(file_path.trim()).map_err(|e| e.to_string())?;
    let mut imported: AppSettings = serde_json::from_str(&data).map_err(|e| e.to_string())?;
    if imported.service_executable.trim().is_empty() {
        imported.service_executable = default_service_executable_path(&app);
    }
    {
        let mut lock = state
            .settings
            .lock()
            .map_err(|_| "settings lock poisoned".to_string())?;
        *lock = imported.clone();
    }
    let _ = apply_third_party_to_remote(&imported);
    if let Ok(catalog) = provider_catalog_with_settings(&imported) {
        let merged = merge_catalog_models(&catalog);
        if !merged.is_empty() {
            imported.available_models = merged;
        }
    }
    save_settings_file(&app, &imported)?;
    {
        let mut lock = state
            .settings
            .lock()
            .map_err(|_| "settings lock poisoned".to_string())?;
        *lock = imported;
    }
    Ok("ok".to_string())
}

#[tauri::command]
pub(crate) fn get_autostart_enabled(app: AppHandle) -> Result<bool, String> {
    app.autolaunch().is_enabled().map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) fn set_autostart_enabled(app: AppHandle, enabled: bool) -> Result<(), String> {
    if enabled {
        app.autolaunch().enable().map_err(|e| e.to_string())?;
    } else {
        app.autolaunch().disable().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub(crate) fn set_service_port(
    app: AppHandle,
    state: tauri::State<AppState>,
    port: u16,
) -> Result<(), String> {
    if port == 0 {
        return Err("port must be between 1 and 65535".to_string());
    }
    let mut next = state
        .settings
        .lock()
        .map_err(|_| "settings lock poisoned".to_string())?
        .clone();
    next.port = port;
    {
        let mut lock = state
            .settings
            .lock()
            .map_err(|_| "settings lock poisoned".to_string())?;
        *lock = next.clone();
    }
    save_settings_file(&app, &next)?;
    let _ = write_runtime_proxy_config(&app, &next);
    Ok(())
}

#[tauri::command]
pub(crate) fn set_logging_to_file_fast(
    app: AppHandle,
    state: tauri::State<AppState>,
    value: bool,
) -> Result<(), String> {
    let mut next = state
        .settings
        .lock()
        .map_err(|_| "settings lock poisoned".to_string())?
        .clone();
    put_bool_to_management(&next, "v0/management/logging-to-file", value)?;
    next.logging_to_file = value;
    {
        let mut lock = state
            .settings
            .lock()
            .map_err(|_| "settings lock poisoned".to_string())?;
        *lock = next.clone();
    }
    save_settings_file(&app, &next)
}

#[tauri::command]
pub(crate) fn set_request_log_fast(
    app: AppHandle,
    state: tauri::State<AppState>,
    value: bool,
) -> Result<(), String> {
    let mut next = state
        .settings
        .lock()
        .map_err(|_| "settings lock poisoned".to_string())?
        .clone();
    put_bool_to_management(&next, "v0/management/request-log", value)?;
    next.request_log = value;
    {
        let mut lock = state
            .settings
            .lock()
            .map_err(|_| "settings lock poisoned".to_string())?;
        *lock = next.clone();
    }
    save_settings_file(&app, &next)
}

#[tauri::command]
pub(crate) fn set_usage_statistics_enabled_fast(
    app: AppHandle,
    state: tauri::State<AppState>,
    value: bool,
) -> Result<(), String> {
    let mut next = state
        .settings
        .lock()
        .map_err(|_| "settings lock poisoned".to_string())?
        .clone();
    put_bool_to_management(&next, "v0/management/usage-statistics-enabled", value)?;
    next.usage_statistics_enabled = value;
    {
        let mut lock = state
            .settings
            .lock()
            .map_err(|_| "settings lock poisoned".to_string())?;
        *lock = next.clone();
    }
    save_settings_file(&app, &next)
}
