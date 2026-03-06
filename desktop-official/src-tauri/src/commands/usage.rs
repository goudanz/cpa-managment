use crate::*;

#[tauri::command]
pub(crate) fn get_usage_snapshot(
    state: tauri::State<AppState>,
) -> Result<serde_json::Value, String> {
    let settings = state
        .settings
        .lock()
        .map_err(|_| "settings lock poisoned".to_string())?
        .clone();

    let value = match management_request_json(
        &settings,
        reqwest::Method::GET,
        "v0/management/usage",
        None,
        2,
    ) {
        Ok(v) => v,
        Err(_) => serde_json::json!({}),
    };

    Ok(value)
}

#[tauri::command]
pub(crate) fn export_usage_file(
    state: tauri::State<AppState>,
    file_path: String,
) -> Result<String, String> {
    let settings = state
        .settings
        .lock()
        .map_err(|_| "settings lock poisoned".to_string())?
        .clone();

    let text = management_request_text(
        &settings,
        reqwest::Method::GET,
        "v0/management/usage/export",
        None,
        20,
    )?;
    fs::write(file_path.trim(), text).map_err(|e| e.to_string())?;
    Ok("ok".to_string())
}

#[tauri::command]
pub(crate) fn import_usage_file(
    state: tauri::State<AppState>,
    file_path: String,
) -> Result<String, String> {
    let settings = state
        .settings
        .lock()
        .map_err(|_| "settings lock poisoned".to_string())?
        .clone();
    let body = fs::read_to_string(file_path.trim()).map_err(|e| e.to_string())?;

    let _ = management_request_json(
        &settings,
        reqwest::Method::POST,
        "v0/management/usage/import",
        Some(body),
        20,
    )?;
    Ok("ok".to_string())
}

#[tauri::command]
pub(crate) fn export_usage_csv_file(
    state: tauri::State<AppState>,
    file_path: String,
) -> Result<String, String> {
    let settings = state
        .settings
        .lock()
        .map_err(|_| "settings lock poisoned".to_string())?
        .clone();

    let value = match management_request_json(
        &settings,
        reqwest::Method::GET,
        "v0/management/usage",
        None,
        2,
    ) {
        Ok(v) => v,
        Err(_) => serde_json::json!({}),
    };

    let usage = value
        .get("usage")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));

    let mut lines: Vec<String> = Vec::new();
    lines.push("scope,key,requests,tokens,success,failure".to_string());

    let total_requests = usage
        .get("total_requests")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    let success_count = usage
        .get("success_count")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    let failure_count = usage
        .get("failure_count")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    let total_tokens = usage
        .get("total_tokens")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    lines.push(format!(
        "total,all,{},{},{},{}",
        total_requests, total_tokens, success_count, failure_count
    ));

    if let Some(apis) = usage.get("apis").and_then(|v| v.as_object()) {
        for (api_name, api_val) in apis {
            let api_req = api_val
                .get("total_requests")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            let api_tok = api_val
                .get("total_tokens")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            lines.push(format!(
                "api,{}, {},{},,",
                api_name.replace(',', "_"),
                api_req,
                api_tok
            ));

            if let Some(models) = api_val.get("models").and_then(|v| v.as_object()) {
                for (model_name, model_val) in models {
                    let req = model_val
                        .get("total_requests")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0);
                    let tok = model_val
                        .get("total_tokens")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0);
                    lines.push(format!(
                        "model,{}::{},{},{},,",
                        api_name.replace(',', "_"),
                        model_name.replace(',', "_"),
                        req,
                        tok
                    ));
                }
            }
        }
    }

    let csv = lines.join("\n");
    fs::write(file_path.trim(), csv).map_err(|e| e.to_string())?;
    Ok("ok".to_string())
}

#[tauri::command]
pub(crate) fn get_usage_summary(
    state: tauri::State<AppState>,
) -> Result<UsageSimpleSummary, String> {
    let settings = state
        .settings
        .lock()
        .map_err(|_| "settings lock poisoned".to_string())?
        .clone();

    let value = match management_request_json(
        &settings,
        reqwest::Method::GET,
        "v0/management/usage",
        None,
        2,
    ) {
        Ok(v) => v,
        Err(_) => {
            return Ok(UsageSimpleSummary {
                total_requests: 0,
                success_count: 0,
                failure_count: 0,
                total_tokens: 0,
                top_models: Vec::new(),
            })
        }
    };

    let usage = value
        .get("usage")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));

    let total_requests = usage
        .get("total_requests")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    let success_count = usage
        .get("success_count")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    let failure_count = usage
        .get("failure_count")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    let total_tokens = usage
        .get("total_tokens")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);

    let mut top_models: Vec<UsageModelSummary> = Vec::new();
    if let Some(apis) = usage.get("apis").and_then(|v| v.as_object()) {
        for api_val in apis.values() {
            if let Some(models) = api_val.get("models").and_then(|v| v.as_object()) {
                for (model_name, model_val) in models {
                    let requests = model_val
                        .get("total_requests")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0);
                    let tokens = model_val
                        .get("total_tokens")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0);
                    if requests == 0 && tokens == 0 {
                        continue;
                    }
                    top_models.push(UsageModelSummary {
                        model: model_name.clone(),
                        requests,
                        tokens,
                    });
                }
            }
        }
    }

    top_models.sort_by(|a, b| {
        b.requests
            .cmp(&a.requests)
            .then_with(|| b.tokens.cmp(&a.tokens))
    });
    if top_models.len() > 5 {
        top_models.truncate(5);
    }

    Ok(UsageSimpleSummary {
        total_requests,
        success_count,
        failure_count,
        total_tokens,
        top_models,
    })
}
