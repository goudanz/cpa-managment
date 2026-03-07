use crate::*;
use serde::Serialize;
use serde_yaml::{Mapping, Value};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ManagementSecretStatus {
    pub(crate) needs_init: bool,
    pub(crate) config_path: String,
}

fn resolve_config_path(app: &AppHandle, settings: &AppSettings) -> Result<PathBuf, String> {
    if !settings.service_config_path.trim().is_empty() {
        return Ok(PathBuf::from(settings.service_config_path.trim()));
    }
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("cliproxy-config.yaml"))
}

fn find_available_port(host: &str, start: u16) -> Option<u16> {
    let mut port = if start == 0 { 1 } else { start };
    for _ in 0..200 {
        if !can_connect(host, port) {
            return Some(port);
        }
        if port == u16::MAX {
            break;
        }
        port = port.saturating_add(1);
    }
    None
}

fn read_runtime_host_port_from_config(path: &PathBuf) -> Option<(String, u16)> {
    let raw = fs::read_to_string(path).ok()?;
    #[derive(serde::Deserialize)]
    struct RuntimeCfg {
        host: Option<String>,
        port: Option<u16>,
    }
    let cfg: RuntimeCfg = serde_yaml::from_str(&raw).ok()?;
    let host = cfg.host.unwrap_or_else(|| "127.0.0.1".to_string());
    let port = cfg.port?;
    Some((host, port))
}

fn read_yaml(path: &PathBuf) -> Result<Value, String> {
    if !path.exists() {
        return Ok(Value::Mapping(Mapping::new()));
    }
    let raw = fs::read_to_string(path).map_err(|e| e.to_string())?;
    if raw.trim().is_empty() {
        return Ok(Value::Mapping(Mapping::new()));
    }
    serde_yaml::from_str::<Value>(&raw).map_err(|e| e.to_string())
}

fn ensure_mapping(value: &mut Value) -> &mut Mapping {
    if !matches!(value, Value::Mapping(_)) {
        *value = Value::Mapping(Mapping::new());
    }
    match value {
        Value::Mapping(map) => map,
        _ => unreachable!(),
    }
}

fn read_secret_from_value(root: &Value) -> String {
    root.get("remote-management")
        .and_then(|v| v.get("secret-key"))
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn read_secret_written(path: &PathBuf) -> Result<String, String> {
    let yaml = read_yaml(path)?;
    Ok(read_secret_from_value(&yaml))
}

#[tauri::command]
pub(crate) fn get_management_secret_status(
    app: AppHandle,
    state: tauri::State<AppState>,
) -> Result<ManagementSecretStatus, String> {
    let mut settings = state
        .settings
        .lock()
        .map_err(|_| "settings lock poisoned".to_string())?
        .clone();
    let cfg_path = resolve_config_path(&app, &settings)?;
    if !cfg_path.exists() {
        let _ = write_runtime_proxy_config(&app, &settings)?;
    }
    if let Some((cfg_host, cfg_port)) = read_runtime_host_port_from_config(&cfg_path) {
        if settings.host != cfg_host || settings.port != cfg_port {
            settings.host = cfg_host;
            settings.port = cfg_port;
            {
                let mut lock = state
                    .settings
                    .lock()
                    .map_err(|_| "settings lock poisoned".to_string())?;
                *lock = settings.clone();
            }
            save_settings_file(&app, &settings)?;
        }
    }
    let yaml = read_yaml(&cfg_path)?;
    let secret = read_secret_from_value(&yaml);
    Ok(ManagementSecretStatus {
        needs_init: secret.is_empty(),
        config_path: cfg_path.to_string_lossy().to_string(),
    })
}

#[tauri::command]
pub(crate) fn initialize_management_secret(
    app: AppHandle,
    state: tauri::State<AppState>,
    secret: String,
) -> Result<(), String> {
    let raw_secret = secret.trim().to_string();
    if raw_secret.len() < 6 {
        return Err("management key must be at least 6 characters".to_string());
    }

    let mut settings = state
        .settings
        .lock()
        .map_err(|_| "settings lock poisoned".to_string())?
        .clone();

    let cfg_path = resolve_config_path(&app, &settings)?;
    if !cfg_path.exists() {
        let _ = write_runtime_proxy_config(&app, &settings)?;
    }
    if let Some((cfg_host, cfg_port)) = read_runtime_host_port_from_config(&cfg_path) {
        settings.host = cfg_host;
        settings.port = cfg_port;
    }

    if can_connect(&settings.host, settings.port) {
        if let Some(next_port) = find_available_port(&settings.host, settings.port.saturating_add(1)) {
            settings.port = next_port;
        }
    }

    let mut root = read_yaml(&cfg_path)?;
    let root_map = ensure_mapping(&mut root);
    let rm_entry = root_map
        .entry(Value::String("remote-management".to_string()))
        .or_insert_with(|| Value::Mapping(Mapping::new()));
    let rm_map = ensure_mapping(rm_entry);

    let hashed = bcrypt::hash(&raw_secret, bcrypt::DEFAULT_COST).map_err(|e| e.to_string())?;
    rm_map.insert(
        Value::String("secret-key".to_string()),
        Value::String(hashed),
    );

    let serialized = serde_yaml::to_string(&root).map_err(|e| e.to_string())?;
    fs::write(&cfg_path, serialized).map_err(|e| e.to_string())?;

    settings.management_password = raw_secret;
    {
        let mut lock = state
            .settings
            .lock()
            .map_err(|_| "settings lock poisoned".to_string())?;
        *lock = settings.clone();
    }
    save_settings_file(&app, &settings)?;

    {
        let mut runtime = state
            .runtime
            .lock()
            .map_err(|_| "runtime lock poisoned".to_string())?;
        if let Some(mut child) = runtime.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        set_runtime_stage(&mut runtime, "BOOTING");
    }

    let fallback_exe = default_service_executable_path(&app);
    let resolved_exe = {
        let configured = settings.service_executable.trim();
        if configured.is_empty() {
            fallback_exe
        } else {
            let configured_path = PathBuf::from(configured);
            if configured_path.exists() {
                configured.to_string()
            } else {
                fallback_exe
            }
        }
    };

    let mut cmd = Command::new(&resolved_exe);
    cmd.arg("-config")
        .arg(cfg_path.to_string_lossy().to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    let child = cmd.spawn().map_err(|e| e.to_string())?;

    {
        let mut runtime = state
            .runtime
            .lock()
            .map_err(|_| "runtime lock poisoned".to_string())?;
        runtime.started_at_unix = now_unix();
        runtime.last_error.clear();
        runtime.child = Some(child);
    }

    let written_secret = read_secret_written(&cfg_path)?;
    if written_secret.trim().is_empty() {
        let mut runtime = state
            .runtime
            .lock()
            .map_err(|_| "runtime lock poisoned".to_string())?;
        set_runtime_stage(&mut runtime, "DEGRADED");
        runtime.error_code = "SECRET_NOT_PERSISTED".to_string();
        runtime.error_detail = "secret-key is still empty after initialization".to_string();
        runtime.last_error = format!("{}: {}", runtime.error_code, runtime.error_detail);
        return Err(runtime.last_error.clone());
    }

    let mut runtime = state
        .runtime
        .lock()
        .map_err(|_| "runtime lock poisoned".to_string())?;
    set_runtime_stage(&mut runtime, "READY");
    Ok(())
}
