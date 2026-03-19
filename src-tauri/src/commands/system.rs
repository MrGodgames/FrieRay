use tauri::State;
use crate::AppState;
use crate::models::settings::AppSettings;

#[tauri::command]
pub async fn save_settings(
    settings: AppSettings,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let config_path = dirs::config_dir()
        .unwrap_or_default()
        .join("frieray/settings.json");

    let json = serde_json::to_string_pretty(&settings)
        .map_err(|e| format!("Serialization error: {}", e))?;

    std::fs::write(&config_path, json)
        .map_err(|e| format!("Failed to save settings: {}", e))?;

    let mut current = state.settings.lock().await;
    *current = settings;

    Ok(())
}

/// Install TUN helper (asks password once)
#[tauri::command]
pub async fn install_tun_helper(state: State<'_, AppState>) -> Result<(), String> {
    state.logs.add("info", "Установка TUN helper (требуется пароль)...").await;
    match state.tun.install_helper().await {
        Ok(()) => {
            state.logs.add("success", "TUN helper установлен — пароль больше не потребуется").await;
            Ok(())
        }
        Err(e) => {
            state.logs.add("error", &format!("TUN helper: {}", e)).await;
            Err(e)
        }
    }
}

/// Check if TUN helper is already installed
#[tauri::command]
pub async fn is_tun_ready(state: State<'_, AppState>) -> Result<bool, String> {
    Ok(state.tun.is_helper_installed())
}

#[tauri::command]
pub async fn load_settings(state: State<'_, AppState>) -> Result<AppSettings, String> {
    let config_path = dirs::config_dir()
        .unwrap_or_default()
        .join("frieray/settings.json");

    if config_path.exists() {
        let json = std::fs::read_to_string(&config_path)
            .map_err(|e| format!("Failed to read settings: {}", e))?;
        let settings: AppSettings = serde_json::from_str(&json)
            .map_err(|e| format!("Settings parse error: {}", e))?;

        let mut current = state.settings.lock().await;
        *current = settings.clone();

        Ok(settings)
    } else {
        let settings = AppSettings::default();
        let mut current = state.settings.lock().await;
        *current = settings.clone();
        Ok(settings)
    }
}

/// List installed applications (macOS)
#[tauri::command]
pub fn get_installed_apps() -> Result<Vec<InstalledApp>, String> {
    let mut apps = Vec::new();

    #[cfg(target_os = "macos")]
    {
        let app_dirs = vec![
            std::path::PathBuf::from("/Applications"),
            dirs::home_dir().unwrap_or_default().join("Applications"),
        ];

        for dir in app_dirs {
            if let Ok(entries) = std::fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().map_or(false, |e| e == "app") {
                        let name = path
                            .file_stem()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string();

                        apps.push(InstalledApp {
                            name,
                            path: path.to_string_lossy().to_string(),
                            bundle_id: get_macos_bundle_id(&path),
                        });
                    }
                }
            }
        }

        apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    }

    Ok(apps)
}

#[cfg(target_os = "macos")]
fn get_macos_bundle_id(app_path: &std::path::Path) -> Option<String> {
    let plist_path = app_path.join("Contents/Info.plist");
    if let Ok(content) = std::fs::read_to_string(&plist_path) {
        // Simple extraction — look for CFBundleIdentifier
        if let Some(pos) = content.find("CFBundleIdentifier") {
            if let Some(start) = content[pos..].find("<string>") {
                if let Some(end) = content[pos + start..].find("</string>") {
                    let bundle_id = &content[pos + start + 8..pos + start + end];
                    return Some(bundle_id.to_string());
                }
            }
        }
    }
    None
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InstalledApp {
    pub name: String,
    pub path: String,
    pub bundle_id: Option<String>,
}
