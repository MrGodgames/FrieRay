use crate::models::settings::AppSettings;
use crate::utils::storage;
use crate::AppState;
use tauri::{AppHandle, State};

#[tauri::command]
pub async fn save_settings(
    settings: AppSettings,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    sync_launch_agent(settings.general.launch_at_login)?;

    storage::save_app_settings(&settings)?;

    let mut current = state.settings.lock().await;
    *current = settings;
    drop(current);

    let _ = crate::core::tray::refresh_tray_async(&app).await;
    Ok(())
}

#[cfg(target_os = "macos")]
fn sync_launch_agent(enabled: bool) -> Result<(), String> {
    let launch_agents_dir = dirs::home_dir()
        .ok_or_else(|| "Не удалось определить домашнюю директорию".to_string())?
        .join("Library/LaunchAgents");
    std::fs::create_dir_all(&launch_agents_dir)
        .map_err(|e| format!("Не удалось создать LaunchAgents: {}", e))?;

    let plist_path = launch_agents_dir.join("com.dreamsoftware.frieray.plist");
    if !enabled {
        if plist_path.exists() {
            std::fs::remove_file(&plist_path)
                .map_err(|e| format!("Не удалось удалить login item: {}", e))?;
        }
        return Ok(());
    }

    let executable =
        std::env::current_exe().map_err(|e| format!("Не удалось определить путь app: {}", e))?;
    let program_args = if let Some(bundle_path) = app_bundle_path(&executable) {
        format!(
            "<array>\n    <string>/usr/bin/open</string>\n    <string>{}</string>\n  </array>",
            xml_escape(&bundle_path.to_string_lossy())
        )
    } else {
        format!(
            "<array>\n    <string>{}</string>\n  </array>",
            xml_escape(&executable.to_string_lossy())
        )
    };

    let plist = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>com.dreamsoftware.frieray</string>
  <key>ProgramArguments</key>
  {program_args}
  <key>RunAtLoad</key>
  <true/>
  <key>KeepAlive</key>
  <false/>
</dict>
</plist>
"#
    );

    std::fs::write(&plist_path, plist)
        .map_err(|e| format!("Не удалось записать login item: {}", e))?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn app_bundle_path(executable: &std::path::Path) -> Option<std::path::PathBuf> {
    let macos_dir = executable.parent()?;
    if macos_dir.file_name()? != "MacOS" {
        return None;
    }
    let contents_dir = macos_dir.parent()?;
    if contents_dir.file_name()? != "Contents" {
        return None;
    }
    let bundle_dir = contents_dir.parent()?;
    if bundle_dir.extension()? == "app" {
        Some(bundle_dir.to_path_buf())
    } else {
        None
    }
}

#[cfg(target_os = "macos")]
fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Install TUN helper (asks password once)
#[tauri::command]
pub async fn install_tun_helper(state: State<'_, AppState>) -> Result<(), String> {
    state
        .logs
        .add("info", "Установка TUN helper (требуется пароль)...")
        .await;
    match state.tun.install_helper().await {
        Ok(()) => {
            state
                .logs
                .add(
                    "success",
                    "TUN helper установлен — пароль больше не потребуется",
                )
                .await;
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
    let settings = storage::load_app_settings();
    let mut current = state.settings.lock().await;
    *current = settings.clone();
    Ok(settings)
}

#[tauri::command]
pub fn show_main_window(app: AppHandle) -> Result<(), String> {
    crate::core::tray::show_main_window(&app)
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
