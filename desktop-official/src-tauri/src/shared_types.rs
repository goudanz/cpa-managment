use std::process::Child;
use std::sync::atomic::AtomicU64;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::management_request_json;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct KeyPolicy {
    pub(crate) api_key: String,
    pub(crate) models: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub(crate) struct AppSettings {
    pub(crate) host: String,
    pub(crate) port: u16,
    pub(crate) api_key: String,
    pub(crate) management_password: String,
    pub(crate) service_executable: String,
    pub(crate) service_config_path: String,
    pub(crate) openai_codex_oauth_enabled: bool,
    pub(crate) claude_oauth_enabled: bool,
    pub(crate) gemini_oauth_enabled: bool,
    pub(crate) force_model_prefix: bool,
    pub(crate) logging_to_file: bool,
    pub(crate) request_log: bool,
    pub(crate) passthrough_headers: bool,
    pub(crate) usage_statistics_enabled: bool,
    pub(crate) imported_auth_files: Vec<String>,
    pub(crate) available_models: Vec<String>,
    pub(crate) third_party_providers: Vec<ThirdPartyProviderInput>,
    pub(crate) key_policies: Vec<KeyPolicy>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8317,
            api_key: String::new(),
            management_password: String::new(),
            service_executable: "".to_string(),
            service_config_path: String::new(),
            openai_codex_oauth_enabled: true,
            claude_oauth_enabled: true,
            gemini_oauth_enabled: true,
            force_model_prefix: false,
            logging_to_file: false,
            request_log: false,
            passthrough_headers: false,
            usage_statistics_enabled: true,
            imported_auth_files: Vec::new(),
            available_models: vec![
                "gpt-5.3-codex".to_string(),
                "gpt-5.2".to_string(),
                "gpt-4.1".to_string(),
            ],
            third_party_providers: Vec::new(),
            key_policies: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HealthState {
    pub(crate) running: bool,
    pub(crate) listening: bool,
    pub(crate) host: String,
    pub(crate) port: u16,
    pub(crate) base_url: String,
    pub(crate) last_error: String,
    pub(crate) started_at_unix: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OAuthStartResult {
    pub(crate) status: String,
    pub(crate) url: String,
    pub(crate) state: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OAuthPollResult {
    pub(crate) status: String,
    pub(crate) error: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct IFlowCookieAuthResult {
    pub(crate) status: String,
    pub(crate) email: String,
    pub(crate) saved_path: String,
    pub(crate) error: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UsageModelSummary {
    pub(crate) model: String,
    pub(crate) requests: i64,
    pub(crate) tokens: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UsageSimpleSummary {
    pub(crate) total_requests: i64,
    pub(crate) success_count: i64,
    pub(crate) failure_count: i64,
    pub(crate) total_tokens: i64,
    pub(crate) top_models: Vec<UsageModelSummary>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OAuthStatusResult {
    pub(crate) provider: String,
    pub(crate) state: String,
    pub(crate) status: String,
    pub(crate) error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProviderModelEntry {
    pub(crate) name: String,
    pub(crate) alias: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ThirdPartyProviderInput {
    pub(crate) name: String,
    pub(crate) prefix: String,
    pub(crate) base_url: String,
    pub(crate) api_key: String,
    #[serde(default)]
    pub(crate) proxy_url: String,
    pub(crate) models: Vec<String>,
}

impl Default for ThirdPartyProviderInput {
    fn default() -> Self {
        Self {
            name: String::new(),
            prefix: String::new(),
            base_url: String::new(),
            api_key: String::new(),
            proxy_url: String::new(),
            models: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProviderCatalogEntry {
    pub(crate) id: String,
    pub(crate) source: String,
    pub(crate) provider: String,
    pub(crate) status: String,
    pub(crate) auth_index: Option<String>,
    pub(crate) models: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AuthFileEntry {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) provider: String,
    pub(crate) status: String,
    pub(crate) runtime_only: bool,
    pub(crate) auth_index: String,
    pub(crate) email: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ChatTestResult {
    pub(crate) ok: bool,
    pub(crate) status: u16,
    pub(crate) reply: String,
    pub(crate) error: String,
}

#[derive(Default)]
pub(crate) struct ServiceRuntime {
    pub(crate) child: Option<Child>,
    pub(crate) last_error: String,
    pub(crate) started_at_unix: u64,
    pub(crate) stage: String,
    pub(crate) error_code: String,
    pub(crate) error_detail: String,
    pub(crate) stage_changed_at_unix: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RuntimeStatus {
    pub(crate) stage: String,
    pub(crate) error_code: String,
    pub(crate) error_detail: String,
    pub(crate) started_at_unix: u64,
    pub(crate) stage_changed_at_unix: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TaskRecord {
    pub(crate) id: String,
    pub(crate) kind: String,
    pub(crate) status: String,
    pub(crate) started_at_unix: u64,
    pub(crate) ended_at_unix: u64,
    pub(crate) message: String,
}

pub(crate) struct AppState {
    pub(crate) settings: Mutex<AppSettings>,
    pub(crate) runtime: Mutex<ServiceRuntime>,
    pub(crate) tasks: Mutex<Vec<TaskRecord>>,
    pub(crate) task_seq: AtomicU64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProviderQuotaWindow {
    pub(crate) id: String,
    pub(crate) label: String,
    pub(crate) used_percent: f64,
    pub(crate) reset_time: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProviderQuotaResult {
    pub(crate) provider: String,
    pub(crate) ok: bool,
    pub(crate) status_code: u16,
    pub(crate) error: String,
    pub(crate) windows: Vec<ProviderQuotaWindow>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BatchImportResult {
    pub(crate) total: usize,
    pub(crate) imported: Vec<String>,
    pub(crate) failed: Vec<String>,
}

pub(crate) fn dedupe_models(models: Vec<String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for m in models {
        let trimmed = m.trim().to_string();
        if trimmed.is_empty() {
            continue;
        }
        if !out.iter().any(|x| x.eq_ignore_ascii_case(&trimmed)) {
            out.push(trimmed);
        }
    }
    out
}

pub(crate) fn provider_catalog_with_settings(
    settings: &AppSettings,
) -> Result<Vec<ProviderCatalogEntry>, String> {
    let auth_files = management_request_json(
        settings,
        reqwest::Method::GET,
        "v0/management/auth-files",
        None,
        5,
    )
    .unwrap_or_else(|_| serde_json::json!({"files": []}));

    let mut entries: Vec<ProviderCatalogEntry> = Vec::new();

    if let Some(files) = auth_files.get("files").and_then(|v| v.as_array()) {
        for f in files {
            let name = f
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            if name.trim().is_empty() {
                continue;
            }
            let provider = f
                .get("provider")
                .or_else(|| f.get("type"))
                .and_then(|v| v.as_str())
                .unwrap_or("oauth")
                .to_string();
            let status = f
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
            let auth_index = f
                .get("auth_index")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let models_resp = management_request_json(
                settings,
                reqwest::Method::GET,
                &format!("v0/management/auth-files/models?name={}", name),
                None,
                5,
            )
            .unwrap_or_else(|_| serde_json::json!({"models": []}));
            let mut models: Vec<String> = Vec::new();
            if let Some(arr) = models_resp.get("models").and_then(|v| v.as_array()) {
                for m in arr {
                    if let Some(id) = m.get("id").and_then(|v| v.as_str()) {
                        models.push(id.to_string());
                    }
                }
            }

            entries.push(ProviderCatalogEntry {
                id: name.clone(),
                source: "auth-file".to_string(),
                provider,
                status,
                auth_index,
                models: dedupe_models(models),
            });
        }
    }

    for p in &settings.third_party_providers {
        let mut source_models = dedupe_models(p.models.clone());
        if source_models.is_empty() {
            source_models.push("gpt-4.1".to_string());
        }
        entries.push(ProviderCatalogEntry {
            id: p.name.clone(),
            source: "third-party".to_string(),
            provider: "openai-compatibility".to_string(),
            status: "configured".to_string(),
            auth_index: None,
            models: source_models,
        });
    }

    Ok(entries)
}

pub(crate) fn auth_files_with_settings(
    settings: &AppSettings,
) -> Result<Vec<AuthFileEntry>, String> {
    let value = management_request_json(
        settings,
        reqwest::Method::GET,
        "v0/management/auth-files",
        None,
        8,
    )?;

    let mut out: Vec<AuthFileEntry> = Vec::new();
    if let Some(files) = value.get("files").and_then(|v| v.as_array()) {
        for item in files {
            let id = item
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .trim()
                .to_string();
            let name = item
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .trim()
                .to_string();
            let provider = item
                .get("provider")
                .or_else(|| item.get("type"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .trim()
                .to_string();
            let status = item
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .trim()
                .to_string();
            let runtime_only = item
                .get("runtime_only")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let auth_index = item
                .get("auth_index")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .trim()
                .to_string();
            let email = item
                .get("email")
                .or_else(|| item.get("account"))
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .trim()
                .to_string();

            let final_name = if name.is_empty() { id.clone() } else { name };
            if final_name.is_empty() {
                continue;
            }

            out.push(AuthFileEntry {
                id,
                name: final_name,
                provider,
                status,
                runtime_only,
                auth_index,
                email,
            });
        }
    }

    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(out)
}

fn as_f64(value: Option<&serde_json::Value>) -> Option<f64> {
    match value {
        Some(v) if v.is_number() => v.as_f64(),
        Some(v) if v.is_string() => v.as_str().and_then(|s| s.trim().parse::<f64>().ok()),
        _ => None,
    }
}

fn as_text(value: Option<&serde_json::Value>) -> String {
    value
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .trim()
        .to_string()
}

pub(crate) fn parse_claude_quota_windows(body: &serde_json::Value) -> Vec<ProviderQuotaWindow> {
    let mut out = Vec::new();
    let keys = [
        ("five_hour", "5h"),
        ("seven_day", "7d"),
        ("seven_day_oauth_apps", "7d OAuth Apps"),
        ("seven_day_opus", "7d Opus"),
        ("seven_day_sonnet", "7d Sonnet"),
    ];
    for (key, label) in keys {
        let Some(window) = body.get(key) else {
            continue;
        };
        let used = as_f64(window.get("utilization")).unwrap_or(0.0) * 100.0;
        let reset = as_text(window.get("resets_at"));
        out.push(ProviderQuotaWindow {
            id: key.to_string(),
            label: label.to_string(),
            used_percent: used.clamp(0.0, 100.0),
            reset_time: reset,
        });
    }
    out
}

pub(crate) fn parse_codex_quota_windows(body: &serde_json::Value) -> Vec<ProviderQuotaWindow> {
    let mut out = Vec::new();
    let pairs = [
        ("rate_limit", "5h"),
        ("code_review_rate_limit", "Code Review 5h"),
    ];
    for (key, label) in pairs {
        let Some(rate) = body.get(key) else {
            continue;
        };
        let primary = rate
            .get("primary_window")
            .or_else(|| rate.get("primaryWindow"));
        if let Some(window) = primary {
            let used = as_f64(
                window
                    .get("used_percent")
                    .or_else(|| window.get("usedPercent")),
            )
            .unwrap_or(0.0);
            let reset = as_f64(
                window
                    .get("reset_after_seconds")
                    .or_else(|| window.get("resetAfterSeconds")),
            )
            .map(|v| format!("{}s", v as i64))
            .unwrap_or_default();
            out.push(ProviderQuotaWindow {
                id: key.to_string(),
                label: label.to_string(),
                used_percent: used.clamp(0.0, 100.0),
                reset_time: reset,
            });
        }
    }

    if out.is_empty() {
        if let Some(items) = body.get("items").and_then(|v| v.as_array()) {
            for item in items {
                let label = as_text(item.get("name").or_else(|| item.get("label")));
                if label.is_empty() {
                    continue;
                }
                let used = as_f64(item.get("used_percent").or_else(|| item.get("usedPercent")))
                    .or_else(|| {
                        let used_raw = as_f64(item.get("used"));
                        let limit_raw = as_f64(item.get("limit"));
                        match (used_raw, limit_raw) {
                            (Some(u), Some(l)) if l > 0.0 => Some((u / l) * 100.0),
                            _ => None,
                        }
                    })
                    .unwrap_or(0.0)
                    .clamp(0.0, 100.0);
                let reset = as_text(
                    item.get("reset_after")
                        .or_else(|| item.get("reset_after_seconds"))
                        .or_else(|| item.get("resetAt"))
                        .or_else(|| item.get("reset_time")),
                );
                out.push(ProviderQuotaWindow {
                    id: label.clone(),
                    label,
                    used_percent: used,
                    reset_time: reset,
                });
            }
        }
    }

    if out.is_empty() {
        if let Some(limit) = body.get("rate_limit") {
            let used = as_f64(limit.get("used_percent").or_else(|| limit.get("usedPercent")))
                .or_else(|| {
                    let used_raw = as_f64(limit.get("used"));
                    let max_raw = as_f64(limit.get("limit").or_else(|| limit.get("max")));
                    match (used_raw, max_raw) {
                        (Some(u), Some(m)) if m > 0.0 => Some((u / m) * 100.0),
                        _ => None,
                    }
                })
                .unwrap_or(0.0)
                .clamp(0.0, 100.0);
            let reset = as_text(
                limit
                    .get("reset_after_seconds")
                    .or_else(|| limit.get("reset_after"))
                    .or_else(|| limit.get("reset_time")),
            );
            out.push(ProviderQuotaWindow {
                id: "rate_limit".to_string(),
                label: "Quota".to_string(),
                used_percent: used,
                reset_time: reset,
            });
        }
    }

    out
}

pub(crate) fn parse_gemini_quota_windows(body: &serde_json::Value) -> Vec<ProviderQuotaWindow> {
    let mut out = Vec::new();
    if let Some(buckets) = body.get("buckets").and_then(|v| v.as_array()) {
        for bucket in buckets {
            let model = as_text(bucket.get("modelId").or_else(|| bucket.get("model_id")));
            if model.is_empty() {
                continue;
            }
            let remaining = as_f64(
                bucket
                    .get("remainingFraction")
                    .or_else(|| bucket.get("remaining_fraction")),
            )
            .unwrap_or(0.0);
            let used = (1.0 - remaining).clamp(0.0, 1.0) * 100.0;
            let reset = as_text(bucket.get("resetTime").or_else(|| bucket.get("reset_time")));
            out.push(ProviderQuotaWindow {
                id: model.clone(),
                label: model,
                used_percent: used,
                reset_time: reset,
            });
        }
    }
    out
}

pub(crate) fn parse_antigravity_quota_windows(
    body: &serde_json::Value,
) -> Vec<ProviderQuotaWindow> {
    let mut out = Vec::new();
    if let Some(models) = body.get("models").and_then(|v| v.as_object()) {
        for (model_id, value) in models {
            let quota = value.get("quotaInfo").or_else(|| value.get("quota_info"));
            let remaining = quota
                .and_then(|q| {
                    as_f64(
                        q.get("remainingFraction")
                            .or_else(|| q.get("remaining_fraction")),
                    )
                })
                .unwrap_or(0.0);
            let used = (1.0 - remaining).clamp(0.0, 1.0) * 100.0;
            let reset = quota
                .map(|q| as_text(q.get("resetTime").or_else(|| q.get("reset_time"))))
                .unwrap_or_default();
            out.push(ProviderQuotaWindow {
                id: model_id.clone(),
                label: model_id.clone(),
                used_percent: used,
                reset_time: reset,
            });
        }
    }
    out
}

pub(crate) fn parse_kimi_quota_windows(body: &serde_json::Value) -> Vec<ProviderQuotaWindow> {
    let mut out = Vec::new();
    if let Some(limits) = body.get("limits").and_then(|v| v.as_array()) {
        for item in limits {
            let label = as_text(item.get("title").or_else(|| item.get("name")));
            if label.is_empty() {
                continue;
            }
            let detail = item.get("detail").unwrap_or(item);
            let used = as_f64(detail.get("used")).unwrap_or(0.0);
            let limit = as_f64(detail.get("limit")).unwrap_or(0.0);
            let used_percent = if limit > 0.0 {
                (used / limit * 100.0).clamp(0.0, 100.0)
            } else {
                0.0
            };
            let reset = as_text(
                detail
                    .get("resetAt")
                    .or_else(|| detail.get("reset_at"))
                    .or_else(|| detail.get("resetTime"))
                    .or_else(|| detail.get("reset_time")),
            );
            out.push(ProviderQuotaWindow {
                id: label.clone(),
                label,
                used_percent,
                reset_time: reset,
            });
        }
    }
    out
}

pub(crate) fn merge_catalog_models(entries: &[ProviderCatalogEntry]) -> Vec<String> {
    let mut models: Vec<String> = Vec::new();
    for entry in entries {
        for m in &entry.models {
            let trimmed = m.trim();
            if trimmed.is_empty() {
                continue;
            }
            if !models.iter().any(|x| x.eq_ignore_ascii_case(trimmed)) {
                models.push(trimmed.to_string());
            }
        }
    }
    models.sort();
    models
}
