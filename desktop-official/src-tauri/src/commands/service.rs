use crate::*;

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

#[tauri::command]
pub(crate) fn get_health(state: tauri::State<AppState>) -> HealthState {
    state_health(&state)
}

#[tauri::command]
pub(crate) fn start_proxy_service(
    app: AppHandle,
    state: tauri::State<AppState>,
) -> Result<(), String> {
    let task_id = begin_task(&state, "start_proxy_service");
    let mut settings = state
        .settings
        .lock()
        .map_err(|_| "settings lock poisoned".to_string())?
        .clone();

    let config_path = if settings.service_config_path.trim().is_empty() {
        let app_cfg_dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
        fs::create_dir_all(&app_cfg_dir).map_err(|e| e.to_string())?;
        app_cfg_dir.join("cliproxy-config.yaml")
    } else {
        PathBuf::from(settings.service_config_path.trim())
    };

    if config_path.exists() {
        if let Some((cfg_host, cfg_port)) = read_runtime_host_port_from_config(&config_path) {
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
    }

    let mut runtime = state
        .runtime
        .lock()
        .map_err(|_| "runtime lock poisoned".to_string())?;

    if let Some(mut child) = runtime.child.take() {
        if child.try_wait().map_err(|e| e.to_string())?.is_none() {
            set_runtime_stage(&mut runtime, "READY");
            runtime.child = Some(child);
            finish_task(&state, &task_id, "success", "service already running");
            return Ok(());
        }
    }

    set_runtime_stage(&mut runtime, "BOOTING");

    if can_connect(&settings.host, settings.port) {
        let can_manage_existing = !settings.management_password.trim().is_empty()
            && management_request_json(
                &settings,
                reqwest::Method::GET,
                "v0/management/config",
                None,
                3,
            )
            .is_ok();

        if can_manage_existing {
            set_runtime_stage(&mut runtime, "READY");
            runtime.error_code.clear();
            runtime.error_detail.clear();
            runtime.last_error.clear();
            finish_task(&state, &task_id, "success", "service already listening and verified");
            return Ok(());
        }

        let Some(next_port) = find_available_port(&settings.host, settings.port.saturating_add(1)) else {
            set_runtime_error(
                &mut runtime,
                "PORT_IN_USE",
                &format!("{}:{} in use and no free fallback port found", settings.host, settings.port),
            );
            finish_task(&state, &task_id, "failed", &runtime.last_error);
            return Err(runtime.last_error.clone());
        };

        settings.port = next_port;
        {
            let mut lock = state
                .settings
                .lock()
                .map_err(|_| "settings lock poisoned".to_string())?;
            *lock = settings.clone();
        }
        save_settings_file(&app, &settings)?;
        let _ = write_runtime_proxy_config(&app, &settings);
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

    let exe_path = PathBuf::from(&resolved_exe);
    if !exe_path.exists() {
        set_runtime_error(
            &mut runtime,
            "BACKEND_NOT_FOUND",
            &format!("backend executable not found: {}", resolved_exe),
        );
        finish_task(
            &state,
            &task_id,
            "failed",
            &format!("BACKEND_NOT_FOUND: {}", resolved_exe),
        );
        return Err(runtime.last_error.clone());
    }

    let mut cmd = Command::new(&resolved_exe);
    let config_path = if config_path.exists() {
        config_path
    } else {
        write_runtime_proxy_config(&app, &settings)?
    };
    cmd.arg("-config")
        .arg(config_path.to_string_lossy().to_string());

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }

    let child = cmd
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| {
            set_runtime_error(&mut runtime, "BACKEND_START_FAILED", &e.to_string());
            finish_task(
                &state,
                &task_id,
                "failed",
                &format!("BACKEND_START_FAILED: {}", e),
            );
            runtime.last_error.clone()
        })?;

    runtime.started_at_unix = now_unix();
    runtime.last_error.clear();
    runtime.child = Some(child);

    let retries = [120_u64, 180, 250, 400, 600, 900];
    let mut healthy = false;
    for delay in retries {
        if can_connect(&settings.host, settings.port) {
            healthy = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(delay));
    }

    if healthy {
        set_runtime_stage(&mut runtime, "READY");
        finish_task(&state, &task_id, "success", "service ready");
    } else {
        set_runtime_stage(&mut runtime, "DEGRADED");
        runtime.error_code = "HEALTH_TIMEOUT".to_string();
        runtime.error_detail = "backend process started but health probe not ready".to_string();
        runtime.last_error = format!("{}: {}", runtime.error_code, runtime.error_detail);
        finish_task(&state, &task_id, "partial", &runtime.last_error);
    }

    Ok(())
}

#[tauri::command]
pub(crate) fn stop_proxy_service(state: tauri::State<AppState>) -> Result<(), String> {
    let task_id = begin_task(&state, "stop_proxy_service");
    let mut runtime = state
        .runtime
        .lock()
        .map_err(|_| "runtime lock poisoned".to_string())?;
    if let Some(mut child) = runtime.child.take() {
        child.kill().map_err(|e| e.to_string())?;
        let _ = child.wait();
    }
    set_runtime_stage(&mut runtime, "STOPPED");
    finish_task(&state, &task_id, "success", "service stopped");
    Ok(())
}

#[tauri::command]
pub(crate) fn copy_base_url(state: tauri::State<AppState>) -> Result<String, String> {
    let settings = state
        .settings
        .lock()
        .map_err(|_| "settings lock poisoned".to_string())?
        .clone();
    let value = local_base_url(&settings);
    let mut clipboard = Clipboard::new().map_err(|e| e.to_string())?;
    clipboard
        .set_text(value.clone())
        .map_err(|e| e.to_string())?;
    Ok(value)
}

#[tauri::command]
pub(crate) fn copy_api_key(state: tauri::State<AppState>) -> Result<String, String> {
    let settings = state
        .settings
        .lock()
        .map_err(|_| "settings lock poisoned".to_string())?
        .clone();
    let value = settings
        .key_policies
        .first()
        .map(|k| k.api_key.clone())
        .unwrap_or_default();
    if value.trim().is_empty() {
        return Err("no key configured yet".to_string());
    }
    let mut clipboard = Clipboard::new().map_err(|e| e.to_string())?;
    clipboard
        .set_text(value.clone())
        .map_err(|e| e.to_string())?;
    Ok(value)
}
