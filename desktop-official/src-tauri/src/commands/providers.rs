use crate::*;

fn management_api_call(
    settings: &AppSettings,
    auth_index: &str,
    method: &str,
    url: &str,
    header: serde_json::Value,
    data: Option<String>,
) -> Result<(u16, serde_json::Value), String> {
    if !can_connect(&settings.host, settings.port) {
        return Ok((
            503,
            serde_json::json!({
                "error": format!(
                    "local management service not reachable at {}",
                    local_base_url(settings)
                )
            }),
        ));
    }

    let payload = serde_json::json!({
        "auth_index": auth_index,
        "method": method,
        "url": url,
        "header": header,
        "data": data.unwrap_or_default(),
    });
    let body = serde_json::to_string(&payload).map_err(|e| e.to_string())?;
    let value = management_request_json(
        settings,
        reqwest::Method::POST,
        "v0/management/api-call",
        Some(body),
        75,
    )
    .map_err(|e| {
        if e.starts_with("MGMT_HTTP_502") {
            return format!(
                "UPSTREAM_UNREACHABLE: provider endpoint failed via local management ({})",
                e
            );
        }
        e
    })?;
    let status_code = value
        .get("status_code")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u16;
    let body_text = value
        .get("body")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    let parsed = serde_json::from_str::<serde_json::Value>(&body_text)
        .unwrap_or_else(|_| serde_json::json!({"raw": body_text}));
    Ok((status_code, parsed))
}

fn normalize_provider_for_quota(provider: &str) -> String {
    let p = provider.trim().to_lowercase();
    match p.as_str() {
        "anthropic" | "claude" => "claude".to_string(),
        "gemini" | "gemini-cli" => "gemini-cli".to_string(),
        _ => p,
    }
}

fn check_codex_quota_with_fallback(
    settings: &AppSettings,
    auth_index: &str,
) -> Result<(u16, serde_json::Value), String> {
    let attempts = codex_quota_attempts();
    run_codex_quota_attempts(&attempts, |url, header| {
        management_api_call(settings, auth_index, "GET", url, header, None)
    })
}

fn codex_quota_attempts() -> Vec<(&'static str, serde_json::Value)> {
    vec![
        (
            "https://chatgpt.com/backend-api/wham/usage",
            serde_json::json!({
                "Authorization": "Bearer $TOKEN$",
                "Content-Type": "application/json",
                "Accept": "application/json",
                "Origin": "https://chatgpt.com",
                "Referer": "https://chatgpt.com/",
                "User-Agent": "codex_cli_rs/0.76.0 (Debian 13.0.0; x86_64) WindowsTerminal",
                "Originator": "codex_cli_rs",
                "Version": "0.76.0"
            }),
        ),
        (
            "https://chatgpt.com/backend-api/usage_limits",
            serde_json::json!({
                "Authorization": "Bearer $TOKEN$",
                "Content-Type": "application/json",
                "Accept": "application/json",
                "Origin": "https://chatgpt.com",
                "Referer": "https://chatgpt.com/",
                "User-Agent": "codex_cli_rs/0.76.0 (Debian 13.0.0; x86_64) WindowsTerminal",
                "Originator": "codex_cli_rs",
                "Version": "0.76.0"
            }),
        ),
        (
            "https://chat.openai.com/backend-api/wham/usage",
            serde_json::json!({
                "Authorization": "Bearer $TOKEN$",
                "Content-Type": "application/json",
                "Accept": "application/json",
                "Origin": "https://chat.openai.com",
                "Referer": "https://chat.openai.com/",
                "User-Agent": "codex_cli_rs/0.76.0 (Debian 13.0.0; x86_64) WindowsTerminal",
                "Originator": "codex_cli_rs",
                "Version": "0.76.0"
            }),
        ),
        (
            "https://chat.openai.com/backend-api/usage_limits",
            serde_json::json!({
                "Authorization": "Bearer $TOKEN$",
                "Content-Type": "application/json",
                "Accept": "application/json",
                "Origin": "https://chat.openai.com",
                "Referer": "https://chat.openai.com/",
                "User-Agent": "codex_cli_rs/0.76.0 (Debian 13.0.0; x86_64) WindowsTerminal",
                "Originator": "codex_cli_rs",
                "Version": "0.76.0"
            }),
        ),
    ]
}

fn run_codex_quota_attempts<F>(
    attempts: &[(&str, serde_json::Value)],
    mut call: F,
) -> Result<(u16, serde_json::Value), String>
where
    F: FnMut(&str, serde_json::Value) -> Result<(u16, serde_json::Value), String>,
{
    let mut last_status: u16 = 0;
    let mut last_body = serde_json::json!({ "error": "request failed" });
    let mut last_err = String::new();

    for (url, header) in attempts {
        match call(url, header.clone()) {
            Ok((status, body)) => {
                last_status = status;
                last_body = body.clone();
                if (200..300).contains(&status) {
                    return Ok((status, body));
                }
            }
            Err(e) => {
                last_err = e;
            }
        }
    }

    if !last_err.is_empty() {
        return Err(last_err);
    }

    Ok((last_status, last_body))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codex_quota_attempts_include_chat_openai_fallback() {
        let attempts = codex_quota_attempts();
        let urls: Vec<&str> = attempts.iter().map(|(url, _)| *url).collect();
        assert!(
            urls.iter().any(|u| u.contains("chat.openai.com/backend-api/wham/usage")),
            "chat.openai fallback should be present"
        );
        assert!(
            urls.iter().any(|u| u.contains("chatgpt.com/backend-api/wham/usage")),
            "chatgpt primary endpoint should be present"
        );
    }

    #[test]
    fn run_codex_quota_attempts_returns_after_successful_fallback() {
        let attempts = codex_quota_attempts();
        let mut calls = 0usize;
        let result = run_codex_quota_attempts(&attempts, |_url, _header| {
            calls += 1;
            if calls == 1 {
                return Err("UPSTREAM_UNREACHABLE: first endpoint failed".to_string());
            }
            Ok((
                200,
                serde_json::json!({
                    "rate_limit": {
                        "primary_window": {
                            "used_percent": 25,
                            "reset_after_seconds": 900
                        }
                    }
                }),
            ))
        })
        .expect("fallback should succeed on second endpoint");

        assert_eq!(calls, 2);
        assert_eq!(result.0, 200);
        assert!(result.1.get("rate_limit").is_some());
    }
}

#[tauri::command]
pub(crate) fn get_auth_files(state: tauri::State<AppState>) -> Result<Vec<AuthFileEntry>, String> {
    let settings = state
        .settings
        .lock()
        .map_err(|_| "settings lock poisoned".to_string())?
        .clone();
    auth_files_with_settings(&settings)
}

#[tauri::command]
pub(crate) async fn check_provider_quota(
    state: tauri::State<'_, AppState>,
    auth_index: String,
    provider: String,
) -> Result<ProviderQuotaResult, String> {
    let settings = state
        .settings
        .lock()
        .map_err(|_| "settings lock poisoned".to_string())?
        .clone();

    let idx = auth_index.trim();
    if idx.is_empty() {
        return Err("auth_index is required".to_string());
    }

    let normalized = normalize_provider_for_quota(&provider);

    let (url, method, header, data) = match normalized.as_str() {
        "claude" => (
            "https://api.anthropic.com/api/oauth/usage".to_string(),
            "GET".to_string(),
            serde_json::json!({
                "Authorization": "Bearer $TOKEN$",
                "Content-Type": "application/json",
                "anthropic-beta": "oauth-2025-04-20"
            }),
            None,
        ),
        "codex" => (
            "".to_string(),
            "".to_string(),
            serde_json::json!({}),
            None,
        ),
        "gemini-cli" => (
            "https://cloudcode-pa.googleapis.com/v1internal:retrieveUserQuota".to_string(),
            "POST".to_string(),
            serde_json::json!({
                "Authorization": "Bearer $TOKEN$",
                "Content-Type": "application/json"
            }),
            Some("{}".to_string()),
        ),
        "antigravity" => (
            "https://daily-cloudcode-pa.googleapis.com/v1internal:fetchAvailableModels".to_string(),
            "POST".to_string(),
            serde_json::json!({
                "Authorization": "Bearer $TOKEN$",
                "Content-Type": "application/json",
                "User-Agent": "antigravity/1.11.5 windows/amd64"
            }),
            Some("{}".to_string()),
        ),
        "kimi" => (
            "https://api.kimi.com/coding/v1/usages".to_string(),
            "GET".to_string(),
            serde_json::json!({
                "Authorization": "Bearer $TOKEN$",
                "Content-Type": "application/json"
            }),
            None,
        ),
        _ => {
            return Ok(ProviderQuotaResult {
                provider: normalized,
                ok: false,
                status_code: 0,
                error: "quota check not supported for this provider yet".to_string(),
                windows: Vec::new(),
            })
        }
    };

    let idx_owned = idx.to_string();
    let settings_owned = settings.clone();
    let method_owned = method.clone();
    let url_owned = url.clone();
    let header_owned = header.clone();
    let data_owned = data.clone();

    let normalized_owned = normalized.clone();
    let api_call_result = tauri::async_runtime::spawn_blocking(move || {
        if normalized_owned == "codex" {
            return check_codex_quota_with_fallback(&settings_owned, &idx_owned);
        }
        management_api_call(
            &settings_owned,
            &idx_owned,
            &method_owned,
            &url_owned,
            header_owned,
            data_owned,
        )
    })
    .await
    .map_err(|e| format!("quota task join error: {}", e))?;

    let (status_code, body) = match api_call_result {
        Ok(v) => v,
        Err(e) => {
            let code = if e.starts_with("UPSTREAM_UNREACHABLE") {
                502
            } else if e.starts_with("MGMT_TIMEOUT") || e.starts_with("MGMT_CONNECT") {
                503
            } else {
                500
            };
            return Ok(ProviderQuotaResult {
                provider: normalized,
                ok: false,
                status_code: code,
                error: e,
                windows: Vec::new(),
            });
        }
    };
    if !(200..300).contains(&status_code) {
        let err_text = body
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| {
                body.get("raw")
                    .and_then(|v| v.as_str())
                    .unwrap_or("request failed")
            })
            .to_string();
        return Ok(ProviderQuotaResult {
            provider: normalized,
            ok: false,
            status_code,
            error: err_text,
            windows: Vec::new(),
        });
    }

    let windows = match normalized.as_str() {
        "claude" => parse_claude_quota_windows(&body),
        "codex" => parse_codex_quota_windows(&body),
        "gemini-cli" => parse_gemini_quota_windows(&body),
        "antigravity" => parse_antigravity_quota_windows(&body),
        "kimi" => parse_kimi_quota_windows(&body),
        _ => Vec::new(),
    };

    Ok(ProviderQuotaResult {
        provider: normalized,
        ok: true,
        status_code,
        error: String::new(),
        windows,
    })
}

#[tauri::command]
pub(crate) fn get_provider_catalog(
    state: tauri::State<AppState>,
) -> Result<Vec<ProviderCatalogEntry>, String> {
    let settings = state
        .settings
        .lock()
        .map_err(|_| "settings lock poisoned".to_string())?
        .clone();
    provider_catalog_with_settings(&settings)
}

#[tauri::command]
pub(crate) fn test_provider_connection(
    state: tauri::State<AppState>,
    base_url: String,
    api_key: String,
) -> Result<serde_json::Value, String> {
    let _settings = state
        .settings
        .lock()
        .map_err(|_| "settings lock poisoned".to_string())?
        .clone();

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(8))
        .build()
        .map_err(|e| e.to_string())?;
    let url = format!("{}/models", base_url.trim_end_matches('/'));
    let resp = client
        .get(url)
        .header("Authorization", format!("Bearer {}", api_key.trim()))
        .send()
        .map_err(|e| map_reqwest_error("PROVIDER_TEST", &e))?;
    let status = resp.status().as_u16();
    let body = resp.text().unwrap_or_default();
    Ok(serde_json::json!({"status": status, "ok": status >= 200 && status < 300, "body": body}))
}

#[tauri::command]
pub(crate) fn fetch_provider_models(
    state: tauri::State<AppState>,
    base_url: String,
    api_key: String,
) -> Result<Vec<String>, String> {
    let _settings = state
        .settings
        .lock()
        .map_err(|_| "settings lock poisoned".to_string())?
        .clone();

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;
    let url = format!("{}/models", base_url.trim_end_matches('/'));
    let value = client
        .get(url)
        .header("Authorization", format!("Bearer {}", api_key.trim()))
        .send()
        .map_err(|e| map_reqwest_error("PROVIDER_FETCH", &e))?
        .json::<serde_json::Value>()
        .map_err(|e| e.to_string())?;

    let mut models: Vec<String> = Vec::new();
    if let Some(arr) = value.get("data").and_then(|v| v.as_array()) {
        for item in arr {
            if let Some(id) = item.get("id").and_then(|v| v.as_str()) {
                models.push(id.to_string());
            }
        }
    }
    Ok(dedupe_models(models))
}

#[tauri::command]
pub(crate) fn chat_test_completion(
    state: tauri::State<AppState>,
    api_key: String,
    model: String,
    prompt: String,
) -> Result<ChatTestResult, String> {
    let settings = state
        .settings
        .lock()
        .map_err(|_| "settings lock poisoned".to_string())?
        .clone();

    let key = api_key.trim();
    let model = model.trim();
    let prompt = prompt.trim();
    if key.is_empty() || model.is_empty() || prompt.is_empty() {
        return Err("api_key, model and prompt are required".to_string());
    }

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;

    let url = format!("{}/v1/chat/completions", local_base_url(&settings));
    let payload = serde_json::json!({
        "model": model,
        "stream": false,
        "messages": [
            {
                "role": "user",
                "content": prompt
            }
        ]
    });

    let resp = client
        .post(url)
        .header("Authorization", format!("Bearer {}", key))
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .map_err(|e| map_reqwest_error("CHAT_TEST", &e))?;

    let status = resp.status().as_u16();
    let value = resp
        .json::<serde_json::Value>()
        .unwrap_or_else(|_| serde_json::json!({}));

    if status < 200 || status >= 300 {
        let err_msg = value
            .get("error")
            .and_then(|v| v.get("message"))
            .and_then(|v| v.as_str())
            .unwrap_or("request failed")
            .to_string();
        return Ok(ChatTestResult {
            ok: false,
            status,
            reply: String::new(),
            error: err_msg,
        });
    }

    let reply = value
        .get("choices")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|v| v.get("message"))
        .and_then(|v| v.get("content"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    Ok(ChatTestResult {
        ok: true,
        status,
        reply,
        error: String::new(),
    })
}
