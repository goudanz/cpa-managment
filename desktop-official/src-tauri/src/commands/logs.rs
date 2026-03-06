use crate::*;

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LogsSnapshot {
    pub(crate) lines: Vec<String>,
    pub(crate) line_count: usize,
    pub(crate) latest_timestamp: i64,
}

#[tauri::command]
pub(crate) fn get_logs_snapshot(
    state: tauri::State<AppState>,
    after: Option<i64>,
    limit: Option<usize>,
) -> Result<LogsSnapshot, String> {
    let settings = state
        .settings
        .lock()
        .map_err(|_| "settings lock poisoned".to_string())?
        .clone();

    let after_value = after.unwrap_or(0);
    let limit_value = limit.unwrap_or(1000).clamp(100, 5000);
    let path = format!(
        "v0/management/logs?after={}&limit={}",
        after_value, limit_value
    );
    let value = match management_request_json(&settings, reqwest::Method::GET, &path, None, 8) {
        Ok(v) => v,
        Err(e) => {
            if e.contains("logging to file disabled") {
                return Ok(LogsSnapshot {
                    lines: vec![
                        "Logging to file is disabled. Enable logging-to-file in config to view logs here."
                            .to_string(),
                    ],
                    line_count: 1,
                    latest_timestamp: after_value,
                });
            }
            return Err(e);
        }
    };

    let mut lines: Vec<String> = Vec::new();
    if let Some(arr) = value.get("lines").and_then(|v| v.as_array()) {
        lines = arr
            .iter()
            .filter_map(|item| item.as_str().map(|s| s.to_string()))
            .collect();
    }

    Ok(LogsSnapshot {
        lines,
        line_count: value
            .get("line-count")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize,
        latest_timestamp: value
            .get("latest-timestamp")
            .and_then(|v| v.as_i64())
            .unwrap_or(after_value),
    })
}

#[tauri::command]
pub(crate) fn clear_logs(state: tauri::State<AppState>) -> Result<(), String> {
    let settings = state
        .settings
        .lock()
        .map_err(|_| "settings lock poisoned".to_string())?
        .clone();
    let _ = management_request_json(
        &settings,
        reqwest::Method::DELETE,
        "v0/management/logs",
        None,
        8,
    )?;
    Ok(())
}
