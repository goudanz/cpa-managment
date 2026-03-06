use std::fs;
use std::net::{SocketAddr, TcpStream};
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Manager};

use crate::{AppSettings, AppState, HealthState, ProviderModelEntry, RuntimeStatus, TaskRecord};

pub(crate) fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub(crate) fn create_task_id(state: &AppState) -> String {
    let seq = state.task_seq.fetch_add(1, Ordering::Relaxed);
    format!("{}-{}", now_unix(), seq)
}

pub(crate) fn begin_task(state: &AppState, kind: &str) -> String {
    let id = create_task_id(state);
    let task = TaskRecord {
        id: id.clone(),
        kind: kind.to_string(),
        status: "running".to_string(),
        started_at_unix: now_unix(),
        ended_at_unix: 0,
        message: String::new(),
    };
    if let Ok(mut tasks) = state.tasks.lock() {
        tasks.push(task);
        if tasks.len() > 200 {
            let drop_n = tasks.len() - 200;
            tasks.drain(0..drop_n);
        }
    }
    id
}

pub(crate) fn finish_task(state: &AppState, id: &str, status: &str, message: &str) {
    if let Ok(mut tasks) = state.tasks.lock() {
        if let Some(item) = tasks.iter_mut().rev().find(|t| t.id == id) {
            item.status = status.to_string();
            item.ended_at_unix = now_unix();
            item.message = message.to_string();
        }
    }
}

pub(crate) fn set_runtime_stage(runtime: &mut crate::ServiceRuntime, stage: &str) {
    runtime.stage = stage.to_string();
    runtime.stage_changed_at_unix = now_unix();
    if stage != "ERROR" {
        runtime.error_code.clear();
        runtime.error_detail.clear();
    }
}

pub(crate) fn set_runtime_error(runtime: &mut crate::ServiceRuntime, code: &str, detail: &str) {
    runtime.stage = "ERROR".to_string();
    runtime.error_code = code.to_string();
    runtime.error_detail = detail.to_string();
    runtime.last_error = format!("{}: {}", code, detail);
    runtime.stage_changed_at_unix = now_unix();
}

pub(crate) fn settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("desktop-settings.json"))
}

pub(crate) fn default_service_executable_path(app: &AppHandle) -> String {
    let binary_name = default_service_binary_name();
    if let Ok(path) = app.path().resource_dir() {
        let candidate = path.join("bin").join(binary_name);
        if candidate.exists() {
            return candidate.to_string_lossy().to_string();
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        let dev_candidate = cwd.join("src-tauri").join("bin").join(binary_name);
        if dev_candidate.exists() {
            return dev_candidate.to_string_lossy().to_string();
        }
    }
    binary_name.to_string()
}

fn default_service_binary_name() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "cliproxyapi.exe"
    }

    #[cfg(not(target_os = "windows"))]
    {
        "cliproxyapi"
    }
}

pub(crate) fn load_settings_file(app: &AppHandle) -> Result<AppSettings, String> {
    let path = settings_path(app)?;
    if !path.exists() {
        return Ok(AppSettings::default());
    }
    let data = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut settings: AppSettings = serde_json::from_str(&data).map_err(|e| e.to_string())?;
    // keep management_password empty by default so first-run can require explicit initialization
    if settings.host.trim().is_empty() {
        settings.host = "127.0.0.1".to_string();
    }
    if settings.port == 0 {
        settings.port = 8317;
    }
    if settings.service_executable.trim().is_empty() {
        settings.service_executable = default_service_executable_path(app);
    }
    Ok(settings)
}

pub(crate) fn save_settings_file(app: &AppHandle, settings: &AppSettings) -> Result<(), String> {
    let path = settings_path(app)?;
    let data = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    fs::write(path, data).map_err(|e| e.to_string())
}

pub(crate) fn local_base_url(settings: &AppSettings) -> String {
    format!("http://{}:{}", settings.host, settings.port)
}

pub(crate) fn map_reqwest_error(prefix: &str, err: &reqwest::Error) -> String {
    if err.is_timeout() {
        return format!("{}_TIMEOUT: {}", prefix, err);
    }
    if err.is_connect() {
        return format!("{}_CONNECT: {}", prefix, err);
    }
    format!("{}_TRANSPORT: {}", prefix, err)
}

pub(crate) fn management_request_json(
    settings: &AppSettings,
    method: reqwest::Method,
    path_with_query: &str,
    body: Option<String>,
    timeout_secs: u64,
) -> Result<serde_json::Value, String> {
    let url = format!("{}/{}", local_base_url(settings), path_with_query);
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .build()
        .map_err(|e| format!("MGMT_CLIENT_INIT: {}", e))?;

    let mut req = client.request(method, &url).header(
        "Authorization",
        format!("Bearer {}", settings.management_password),
    );
    if let Some(payload) = body {
        req = req.header("Content-Type", "application/json").body(payload);
    }

    let resp = req.send().map_err(|e| map_reqwest_error("MGMT", &e))?;

    let status = resp.status();
    if !status.is_success() {
        let body_text = resp.text().unwrap_or_default();
        return Err(format!(
            "MGMT_HTTP_{}: {}",
            status.as_u16(),
            body_text.trim()
        ));
    }

    resp.json::<serde_json::Value>()
        .map_err(|e| format!("MGMT_JSON_PARSE: {}", e))
}

pub(crate) fn management_request_text(
    settings: &AppSettings,
    method: reqwest::Method,
    path_with_query: &str,
    body: Option<String>,
    timeout_secs: u64,
) -> Result<String, String> {
    let url = format!("{}/{}", local_base_url(settings), path_with_query);
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .build()
        .map_err(|e| format!("MGMT_CLIENT_INIT: {}", e))?;

    let mut req = client.request(method, &url).header(
        "Authorization",
        format!("Bearer {}", settings.management_password),
    );
    if let Some(payload) = body {
        req = req.header("Content-Type", "application/json").body(payload);
    }

    let resp = req.send().map_err(|e| map_reqwest_error("MGMT", &e))?;

    let status = resp.status();
    let text = resp.text().unwrap_or_default();
    if !status.is_success() {
        return Err(format!("MGMT_HTTP_{}: {}", status.as_u16(), text.trim()));
    }
    Ok(text)
}

pub(crate) fn management_request_bytes(
    settings: &AppSettings,
    method: reqwest::Method,
    path_with_query: &str,
    body: Vec<u8>,
    content_type: &str,
    timeout_secs: u64,
) -> Result<serde_json::Value, String> {
    let url = format!("{}/{}", local_base_url(settings), path_with_query);
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .build()
        .map_err(|e| format!("MGMT_CLIENT_INIT: {}", e))?;

    let resp = client
        .request(method, &url)
        .header(
            "Authorization",
            format!("Bearer {}", settings.management_password),
        )
        .header("Content-Type", content_type)
        .body(body)
        .send()
        .map_err(|e| map_reqwest_error("MGMT", &e))?;

    let status = resp.status();
    if !status.is_success() {
        let body_text = resp.text().unwrap_or_default();
        return Err(format!(
            "MGMT_HTTP_{}: {}",
            status.as_u16(),
            body_text.trim()
        ));
    }

    resp.json::<serde_json::Value>()
        .map_err(|e| format!("MGMT_JSON_PARSE: {}", e))
}

pub(crate) fn usage_snapshot_with_settings(settings: &AppSettings) -> serde_json::Value {
    match management_request_json(
        settings,
        reqwest::Method::GET,
        "v0/management/usage",
        None,
        2,
    ) {
        Ok(v) => v,
        Err(_) => serde_json::json!({}),
    }
}

pub(crate) fn apply_third_party_to_remote(settings: &AppSettings) -> Result<(), String> {
    let mut payload: Vec<serde_json::Value> = Vec::new();
    for item in &settings.third_party_providers {
        let name = item.name.trim();
        let base = item.base_url.trim();
        let key = item.api_key.trim();
        if name.is_empty() || base.is_empty() || key.is_empty() {
            continue;
        }

        let models: Vec<ProviderModelEntry> = crate::dedupe_models(item.models.clone())
            .into_iter()
            .map(|m| ProviderModelEntry {
                name: m.clone(),
                alias: m,
            })
            .collect();

        payload.push(serde_json::json!({
            "name": name,
            "prefix": item.prefix,
            "base-url": base,
            "api-key-entries": [
                {
                    "api-key": key,
                    "proxy-url": item.proxy_url,
                }
            ],
            "models": models,
        }));
    }

    let body = serde_json::to_string(&payload).map_err(|e| e.to_string())?;
    let _ = management_request_json(
        settings,
        reqwest::Method::PUT,
        "v0/management/openai-compatibility",
        Some(body),
        10,
    )?;
    Ok(())
}

pub(crate) fn yaml_quote(value: &str) -> String {
    let escaped = value.replace('"', "''");
    format!("'{}'", escaped)
}

pub(crate) fn write_runtime_proxy_config(
    app: &AppHandle,
    settings: &AppSettings,
) -> Result<PathBuf, String> {
    let mut merged_keys: Vec<String> = Vec::new();
    for policy in &settings.key_policies {
        let key = policy.api_key.trim();
        if key.is_empty() {
            continue;
        }
        if !merged_keys.iter().any(|item| item == key) {
            merged_keys.push(key.to_string());
        }
    }

    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    let auth_dir = dir.join("auths");
    fs::create_dir_all(&auth_dir).map_err(|e| e.to_string())?;

    let cfg_path = dir.join("cliproxy-config.yaml");
    let mut content = String::new();
    content.push_str(&format!("host: {}\n", yaml_quote(settings.host.trim())));
    content.push_str(&format!("port: {}\n", settings.port));
    content.push_str(&format!(
        "force-model-prefix: {}\n",
        settings.force_model_prefix
    ));
    content.push_str(&format!("logging-to-file: {}\n", settings.logging_to_file));
    content.push_str(&format!("request-log: {}\n", settings.request_log));
    content.push_str(&format!(
        "passthrough-headers: {}\n",
        settings.passthrough_headers
    ));
    content.push_str(&format!(
        "usage-statistics-enabled: {}\n",
        settings.usage_statistics_enabled
    ));
    content.push_str("remote-management:\n");
    content.push_str("  allow-remote: false\n");
    let secret_value = settings.management_password.trim();
    if secret_value.is_empty() {
        content.push_str("  secret-key: \"\"\n");
    } else {
        content.push_str(&format!("  secret-key: {}\n", yaml_quote(secret_value)));
    }
    content.push_str("  disable-control-panel: true\n");
    let auth_dir_str = auth_dir.to_string_lossy().replace('\\', "/");
    content.push_str(&format!("auth-dir: {}\n", yaml_quote(&auth_dir_str)));
    content.push_str("api-keys:\n");
    for key in &merged_keys {
        content.push_str(&format!("  - {}\n", yaml_quote(key)));
    }
    content.push_str("api-key-policies:\n");
    for policy in &settings.key_policies {
        let key = policy.api_key.trim();
        if key.is_empty() {
            continue;
        }
        content.push_str(&format!("  - api-key: {}\n", yaml_quote(key)));
        content.push_str("    models:\n");
        for model in &policy.models {
            let trimmed = model.trim();
            if trimmed.is_empty() {
                continue;
            }
            content.push_str(&format!("      - {}\n", yaml_quote(trimmed)));
        }
    }

    fs::write(&cfg_path, content).map_err(|e| e.to_string())?;
    Ok(cfg_path)
}

pub(crate) fn can_connect(host: &str, port: u16) -> bool {
    let addr = format!("{}:{}", host, port);
    let parsed: Result<SocketAddr, _> = addr.parse();
    if let Ok(socket_addr) = parsed {
        return TcpStream::connect_timeout(&socket_addr, Duration::from_millis(200)).is_ok();
    }
    false
}

pub(crate) fn state_health(state: &AppState) -> HealthState {
    let settings = state
        .settings
        .lock()
        .expect("settings lock poisoned")
        .clone();
    let mut runtime = state.runtime.lock().expect("runtime lock poisoned");

    let mut running = false;
    if let Some(mut child) = runtime.child.take() {
        match child.try_wait() {
            Ok(Some(_)) => {
                if runtime.stage != "STOPPED" {
                    set_runtime_stage(&mut runtime, "STOPPED");
                }
                running = false;
            }
            Ok(None) => {
                running = true;
                if can_connect(&settings.host, settings.port) {
                    set_runtime_stage(&mut runtime, "READY");
                }
                runtime.child = Some(child);
            }
            Err(e) => {
                set_runtime_error(&mut runtime, "CHILD_STATE_ERROR", &e.to_string());
                running = false;
            }
        }
    }

    let listening = can_connect(&settings.host, settings.port);

    HealthState {
        running,
        listening,
        host: settings.host.clone(),
        port: settings.port,
        base_url: local_base_url(&settings),
        last_error: runtime.last_error.clone(),
        started_at_unix: runtime.started_at_unix,
    }
}

pub(crate) fn runtime_status_value(state: &AppState) -> RuntimeStatus {
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

pub(crate) fn latest_tasks_value(state: &AppState) -> Vec<TaskRecord> {
    if let Ok(tasks) = state.tasks.lock() {
        return tasks.iter().rev().take(60).cloned().collect();
    }
    Vec::new()
}
