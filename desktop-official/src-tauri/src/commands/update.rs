use crate::*;

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UpdateInfo {
    pub(crate) current_version: String,
    pub(crate) latest_version: String,
    pub(crate) has_update: bool,
    pub(crate) notes: String,
    pub(crate) download_url: String,
    pub(crate) asset_name: String,
}

#[tauri::command]
pub(crate) fn check_update() -> Result<UpdateInfo, String> {
    let current = env!("CARGO_PKG_VERSION").to_string();
    let api = "https://api.github.com/repos/WEP-56/CPAPersonal/releases/latest";
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(12))
        .build()
        .map_err(|e| format!("UPDATE_CLIENT_INIT: {}", e))?;

    let value = client
        .get(api)
        .header("User-Agent", "CPAPersonal-Updater")
        .send()
        .map_err(|e| map_reqwest_error("UPDATE", &e))?
        .json::<serde_json::Value>()
        .map_err(|e| format!("UPDATE_JSON_PARSE: {}", e))?;

    let tag = value
        .get("tag_name")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    if tag.is_empty() {
        return Err("UPDATE_TAG_EMPTY".to_string());
    }

    let latest_clean = tag.trim_start_matches('v').to_string();
    let current_ver = semver::Version::parse(&current).map_err(|e| e.to_string())?;
    let latest_ver = semver::Version::parse(&latest_clean).map_err(|e| e.to_string())?;

    let mut download_url = String::new();
    let mut asset_name = String::new();
    if let Some(assets) = value.get("assets").and_then(|v| v.as_array()) {
        for item in assets {
            let name = item.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let url = item
                .get("browser_download_url")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let low = name.to_lowercase();
            if is_supported_installer_asset(&low) {
                download_url = url.to_string();
                asset_name = name.to_string();
                break;
            }
        }
    }

    Ok(UpdateInfo {
        current_version: current,
        latest_version: latest_clean,
        has_update: latest_ver > current_ver,
        notes: value
            .get("body")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        download_url,
        asset_name,
    })
}

#[tauri::command]
pub(crate) fn download_and_open_update_installer(
    download_url: String,
    file_name: String,
) -> Result<String, String> {
    let target_dir = dirs::download_dir()
        .or_else(|| std::env::current_dir().ok())
        .ok_or_else(|| "DOWNLOAD_DIR_NOT_FOUND".to_string())?;
    let safe_name = file_name.trim();
    if safe_name.is_empty() {
        return Err("INVALID_FILE_NAME".to_string());
    }
    let target = target_dir.join(safe_name);

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(240))
        .build()
        .map_err(|e| format!("UPDATE_CLIENT_INIT: {}", e))?;
    let bytes = client
        .get(download_url.trim())
        .header("User-Agent", "CPAPersonal-Updater")
        .send()
        .map_err(|e| map_reqwest_error("UPDATE", &e))?
        .bytes()
        .map_err(|e| format!("UPDATE_BYTES: {}", e))?;
    fs::write(&target, bytes).map_err(|e| e.to_string())?;

    launch_installer(&target)?;

    Ok(target.to_string_lossy().to_string())
}

fn is_supported_installer_asset(asset_name: &str) -> bool {
    #[cfg(target_os = "windows")]
    {
        asset_name.ends_with(".exe") || asset_name.ends_with(".msi")
    }

    #[cfg(target_os = "macos")]
    {
        asset_name.ends_with(".dmg") || asset_name.ends_with(".pkg") || asset_name.ends_with(".zip")
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        asset_name.ends_with(".appimage") || asset_name.ends_with(".deb") || asset_name.ends_with(".rpm")
    }
}

fn launch_installer(target: &std::path::Path) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(target)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("OPEN_INSTALLER: {}", e))?;
        return Ok(());
    }

    #[cfg(not(target_os = "macos"))]
    {
        Command::new(target)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("OPEN_INSTALLER: {}", e))?;
        Ok(())
    }
}
