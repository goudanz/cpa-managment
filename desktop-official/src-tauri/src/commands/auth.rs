use crate::*;

fn import_auth_file_impl(
    app: &AppHandle,
    state: &tauri::State<AppState>,
    file_path: &str,
) -> Result<String, String> {
    let path = PathBuf::from(file_path.trim());
    if !path.exists() {
        return Err("file does not exist".to_string());
    }
    let file_name = path
        .file_name()
        .and_then(|v| v.to_str())
        .ok_or_else(|| "invalid file name".to_string())?
        .to_string();
    if !file_name.to_ascii_lowercase().ends_with(".json") {
        return Err("only .json auth files are supported".to_string());
    }

    let settings = state
        .settings
        .lock()
        .map_err(|_| "settings lock poisoned".to_string())?
        .clone();

    let mut file = fs::File::open(&path).map_err(|e| e.to_string())?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).map_err(|e| e.to_string())?;

    let _ = management_request_bytes(
        &settings,
        reqwest::Method::POST,
        &format!("v0/management/auth-files?name={}", file_name),
        bytes,
        "application/json",
        20,
    )?;

    let mut next = settings.clone();
    if !next.imported_auth_files.iter().any(|v| v == &file_name) {
        next.imported_auth_files.push(file_name.clone());
        {
            let mut lock = state
                .settings
                .lock()
                .map_err(|_| "settings lock poisoned".to_string())?;
            *lock = next.clone();
        }
        save_settings_file(app, &next)?;
    }

    Ok(file_name)
}

#[tauri::command]
pub(crate) fn import_auth_file(
    app: AppHandle,
    state: tauri::State<AppState>,
    file_path: String,
) -> Result<String, String> {
    import_auth_file_impl(&app, &state, &file_path)
}

#[tauri::command]
pub(crate) fn import_auth_files_batch(
    app: AppHandle,
    state: tauri::State<AppState>,
    file_paths: Vec<String>,
) -> Result<BatchImportResult, String> {
    if file_paths.is_empty() {
        return Ok(BatchImportResult {
            total: 0,
            imported: Vec::new(),
            failed: Vec::new(),
        });
    }

    let mut imported: Vec<String> = Vec::new();
    let mut failed: Vec<String> = Vec::new();

    for file_path in &file_paths {
        match import_auth_file_impl(&app, &state, file_path) {
            Ok(name) => imported.push(name),
            Err(err) => failed.push(format!("{}: {}", file_path, err)),
        }
    }

    Ok(BatchImportResult {
        total: file_paths.len(),
        imported,
        failed,
    })
}

#[tauri::command]
pub(crate) fn remove_auth_file(
    app: AppHandle,
    state: tauri::State<AppState>,
    name: String,
) -> Result<(), String> {
    let file_name = name.trim();
    if file_name.is_empty() {
        return Err("name is empty".to_string());
    }
    let settings = state
        .settings
        .lock()
        .map_err(|_| "settings lock poisoned".to_string())?
        .clone();

    let _ = management_request_json(
        &settings,
        reqwest::Method::DELETE,
        &format!("v0/management/auth-files?name={}", file_name),
        None,
        20,
    )?;

    let mut next = settings.clone();
    next.imported_auth_files.retain(|item| item != file_name);
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
pub(crate) fn patch_auth_file_status(
    state: tauri::State<AppState>,
    name: String,
    disabled: bool,
) -> Result<(), String> {
    let file_name = name.trim();
    if file_name.is_empty() {
        return Err("name is empty".to_string());
    }
    let settings = state
        .settings
        .lock()
        .map_err(|_| "settings lock poisoned".to_string())?
        .clone();

    let payload = serde_json::json!({
        "name": file_name,
        "disabled": disabled,
    });
    let body = serde_json::to_string(&payload).map_err(|e| e.to_string())?;
    let _ = management_request_json(
        &settings,
        reqwest::Method::PATCH,
        "v0/management/auth-files/status",
        Some(body),
        20,
    )?;

    Ok(())
}

#[tauri::command]
pub(crate) fn start_iflow_cookie_auth(
    state: tauri::State<AppState>,
    cookie: String,
) -> Result<IFlowCookieAuthResult, String> {
    let settings = state
        .settings
        .lock()
        .map_err(|_| "settings lock poisoned".to_string())?
        .clone();
    let payload = serde_json::json!({
        "cookie": cookie.trim(),
    });
    let body = serde_json::to_string(&payload).map_err(|e| e.to_string())?;
    let value = management_request_json(
        &settings,
        reqwest::Method::POST,
        "v0/management/iflow-auth-url",
        Some(body),
        30,
    )?;

    Ok(IFlowCookieAuthResult {
        status: value
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("ok")
            .to_string(),
        email: value
            .get("email")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        saved_path: value
            .get("saved_path")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        error: value
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
    })
}

#[tauri::command]
pub(crate) fn start_codex_oauth(state: tauri::State<AppState>) -> Result<OAuthStartResult, String> {
    start_oauth_for_provider(state, "codex")
}

pub(crate) fn start_oauth_for_provider(
    state: tauri::State<AppState>,
    provider: &str,
) -> Result<OAuthStartResult, String> {
    let settings = state
        .settings
        .lock()
        .map_err(|_| "settings lock poisoned".to_string())?
        .clone();
    let endpoint = match provider {
        "codex" => "codex-auth-url",
        "anthropic" => "anthropic-auth-url",
        "claude" => "anthropic-auth-url",
        "gemini" => "gemini-cli-auth-url",
        "gemini-cli" => "gemini-cli-auth-url",
        "antigravity" => "antigravity-auth-url",
        "qwen" => "qwen-auth-url",
        "kimi" => "kimi-auth-url",
        "iflow" => "iflow-auth-url",
        _ => return Err("unsupported oauth provider".to_string()),
    };

    let value = management_request_json(
        &settings,
        reqwest::Method::GET,
        &format!("v0/management/{}?is_webui=true", endpoint),
        None,
        20,
    )?;

    let status = value
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let auth_url = value
        .get("url")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let oauth_state = value
        .get("state")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    if status != "ok" || auth_url.is_empty() || oauth_state.is_empty() {
        return Err(format!("failed to start {} oauth", provider));
    }

    Ok(OAuthStartResult {
        status,
        url: auth_url,
        state: oauth_state,
    })
}

#[tauri::command]
pub(crate) fn start_oauth(
    state: tauri::State<AppState>,
    provider: String,
) -> Result<OAuthStartResult, String> {
    let normalized = provider.trim().to_lowercase();
    start_oauth_for_provider(state, &normalized)
}

#[tauri::command]
pub(crate) fn poll_oauth_status(
    state: tauri::State<AppState>,
    oauth_state: String,
) -> Result<OAuthPollResult, String> {
    let settings = state
        .settings
        .lock()
        .map_err(|_| "settings lock poisoned".to_string())?
        .clone();
    let value = management_request_json(
        &settings,
        reqwest::Method::GET,
        &format!("v0/management/get-auth-status?state={}", oauth_state.trim()),
        None,
        20,
    )?;

    let status = value
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("error")
        .to_string();
    let error = value
        .get("error")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    Ok(OAuthPollResult { status, error })
}

#[tauri::command]
pub(crate) fn poll_multi_oauth_status(
    state: tauri::State<AppState>,
    provider: String,
    oauth_state: String,
) -> Result<OAuthStatusResult, String> {
    let res = poll_oauth_status(state, oauth_state.clone())?;
    Ok(OAuthStatusResult {
        provider,
        state: oauth_state,
        status: res.status,
        error: res.error,
    })
}
