use crate::*;

#[tauri::command]
pub(crate) fn get_runtime_status(state: tauri::State<AppState>) -> RuntimeStatus {
    let runtime = state.runtime.lock().expect("runtime lock poisoned");
    RuntimeStatus {
        stage: if runtime.stage.is_empty() {
            "STOPPED".to_string()
        } else {
            runtime.stage.clone()
        },
        error_code: runtime.error_code.clone(),
        error_detail: runtime.error_detail.clone(),
        started_at_unix: runtime.started_at_unix,
        stage_changed_at_unix: runtime.stage_changed_at_unix,
    }
}

#[tauri::command]
pub(crate) fn export_diagnostics_file(
    app: AppHandle,
    state: tauri::State<AppState>,
    file_path: String,
) -> Result<String, String> {
    let settings = state
        .settings
        .lock()
        .map_err(|_| "settings lock poisoned".to_string())?
        .clone();
    let runtime_json = {
        let runtime = state
            .runtime
            .lock()
            .map_err(|_| "runtime lock poisoned".to_string())?;
        RuntimeStatus {
            stage: if runtime.stage.is_empty() {
                "STOPPED".to_string()
            } else {
                runtime.stage.clone()
            },
            error_code: runtime.error_code.clone(),
            error_detail: runtime.error_detail.clone(),
            started_at_unix: runtime.started_at_unix,
            stage_changed_at_unix: runtime.stage_changed_at_unix,
        }
    };
    let tasks = state
        .tasks
        .lock()
        .map_err(|_| "task lock poisoned".to_string())?
        .clone();

    let health = state_health(&state);
    let usage = usage_snapshot_with_settings(&settings);

    let payload = serde_json::json!({
        "timestamp": now_unix(),
        "appConfigDir": app.path().app_config_dir().ok().map(|p| p.to_string_lossy().to_string()).unwrap_or_default(),
        "settings": settings,
        "runtime": runtime_json,
        "health": health,
        "tasks": tasks,
        "usage": usage,
    });

    let serialized = serde_json::to_string_pretty(&payload).map_err(|e| e.to_string())?;
    fs::write(file_path.trim(), serialized).map_err(|e| e.to_string())?;
    Ok("ok".to_string())
}
