use crate::*;

#[tauri::command]
pub(crate) fn patch_key_policy_models(
    app: AppHandle,
    state: tauri::State<AppState>,
    api_key: String,
    models: Vec<String>,
) -> Result<(), String> {
    let key = api_key.trim();
    if key.is_empty() {
        return Err("api key is empty".to_string());
    }
    let normalized = dedupe_models(models);
    let mut settings = state
        .settings
        .lock()
        .map_err(|_| "settings lock poisoned".to_string())?
        .clone();

    let mut found = false;
    for item in &mut settings.key_policies {
        if item.api_key == key {
            item.models = normalized.clone();
            found = true;
            break;
        }
    }
    if !found {
        return Err("key not found".to_string());
    }
    {
        let mut lock = state
            .settings
            .lock()
            .map_err(|_| "settings lock poisoned".to_string())?;
        *lock = settings.clone();
    }
    save_settings_file(&app, &settings)?;
    let _ = write_runtime_proxy_config(&app, &settings);
    Ok(())
}

#[tauri::command]
pub(crate) fn delete_key_policy(
    app: AppHandle,
    state: tauri::State<AppState>,
    api_key: String,
) -> Result<(), String> {
    let key = api_key.trim();
    if key.is_empty() {
        return Err("api key is empty".to_string());
    }
    let mut settings = state
        .settings
        .lock()
        .map_err(|_| "settings lock poisoned".to_string())?
        .clone();
    settings.key_policies.retain(|k| k.api_key != key);
    {
        let mut lock = state
            .settings
            .lock()
            .map_err(|_| "settings lock poisoned".to_string())?;
        *lock = settings.clone();
    }
    save_settings_file(&app, &settings)?;
    let _ = write_runtime_proxy_config(&app, &settings);
    Ok(())
}
