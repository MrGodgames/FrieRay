use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

use crate::core::config::generate_xray_config;
use crate::models::server::Server;
use crate::models::settings::AppSettings;

/// Manages the Xray-core process lifecycle
pub struct XrayManager {
    process: Arc<Mutex<Option<Child>>>,
    config_path: PathBuf,
}

impl XrayManager {
    pub fn new() -> Self {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("frieray");

        std::fs::create_dir_all(&config_dir).ok();

        Self {
            process: Arc::new(Mutex::new(None)),
            config_path: config_dir.join("xray-config.json"),
        }
    }

    /// Start xray-core with a generated config
    pub async fn start(&self, server: &Server, settings: &AppSettings) -> Result<(), String> {
        // Stop if already running
        self.stop().await.ok(); // Don't fail if stop errors

        // Generate config
        let config = generate_xray_config(server, settings, 10085);
        let config_json =
            serde_json::to_string_pretty(&config).map_err(|e| format!("Config error: {}", e))?;

        std::fs::write(&self.config_path, &config_json)
            .map_err(|e| format!("Write config error: {}", e))?;

        log::info!("Xray config written to {:?}", self.config_path);

        // Find xray binary
        let xray_bin = self.find_xray_binary()?;
        log::info!("Using xray binary: {:?}", xray_bin);

        // Start xray process
        let child = Command::new(&xray_bin)
            .arg("run")
            .arg("-config")
            .arg(&self.config_path)
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| format!("Failed to start xray: {}", e))?;

        log::info!("Xray-core started (PID: {:?})", child.id());

        {
            let mut proc = self.process.lock().await;
            *proc = Some(child);
        }

        // Wait a moment to check if process started successfully
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        // Verify process is still running
        {
            let mut proc = self.process.lock().await;
            if let Some(ref mut child) = *proc {
                match child.try_wait() {
                    Ok(Some(status)) => {
                        // Process exited immediately — read stderr for error message
                        let stderr_msg = if let Some(mut stderr) = child.stderr.take() {
                            use tokio::io::AsyncReadExt;
                            let mut buf = vec![0u8; 4096];
                            match tokio::time::timeout(
                                std::time::Duration::from_secs(2),
                                stderr.read(&mut buf),
                            )
                            .await
                            {
                                Ok(Ok(n)) => String::from_utf8_lossy(&buf[..n]).to_string(),
                                _ => String::new(),
                            }
                        } else {
                            String::new()
                        };

                        *proc = None;
                        let err = if stderr_msg.trim().is_empty() {
                            format!("Xray завершился со статусом: {}", status)
                        } else {
                            format!("Xray ошибка: {}", stderr_msg.trim())
                        };
                        log::error!("{}", err);
                        return Err(err);
                    }
                    Ok(None) => {
                        log::info!("Xray-core is running successfully");
                    }
                    Err(e) => {
                        return Err(format!("Status check error: {}", e));
                    }
                }
            }
        }

        Ok(())
    }

    /// Stop xray-core process  
    pub async fn stop(&self) -> Result<(), String> {
        let mut proc = self.process.lock().await;
        if let Some(mut child) = proc.take() {
            log::info!("Stopping xray-core (PID: {:?})...", child.id());
            // Kill and wait with timeout
            let _ = child.kill().await;
            match tokio::time::timeout(std::time::Duration::from_secs(3), child.wait()).await {
                Ok(_) => log::info!("Xray-core stopped"),
                Err(_) => log::warn!("Xray-core stop timed out, force killed"),
            }
        }
        Ok(())
    }

    /// Check if xray is running
    pub async fn is_running(&self) -> bool {
        let mut proc = self.process.lock().await;
        if let Some(ref mut child) = *proc {
            match child.try_wait() {
                Ok(Some(_)) => {
                    *proc = None;
                    false
                }
                Ok(None) => true,
                Err(_) => false,
            }
        } else {
            false
        }
    }

    /// Find xray binary
    pub fn find_xray_binary(&self) -> Result<PathBuf, String> {
        if let Ok(exe) = std::env::current_exe() {
            let exe_dir = exe.parent().unwrap_or_else(|| std::path::Path::new("."));

            let candidates = vec![
                exe_dir.join("xray"),
                exe_dir.join("xray-aarch64-apple-darwin"),
                exe_dir.join("xray-x86_64-apple-darwin"),
                exe_dir.join("xray-x86_64-pc-windows-msvc.exe"),
                exe_dir.join("../Resources/xray"),
                exe_dir.join("../../binaries/xray"),
                exe_dir.join("../../../src-tauri/binaries/xray"),
            ];

            for path in &candidates {
                if path.exists() {
                    log::info!("Found xray at: {:?}", path);
                    return Ok(path.clone());
                }
            }
        }

        let project_bin = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("binaries/xray");
        if project_bin.exists() {
            return Ok(project_bin);
        }

        let config_bin = dirs::config_dir().unwrap_or_default().join("frieray/xray");
        if config_bin.exists() {
            return Ok(config_bin);
        }

        if let Ok(output) = std::process::Command::new("which").arg("xray").output() {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path.is_empty() {
                    return Ok(PathBuf::from(path));
                }
            }
        }

        Err("Xray-core не найден. Бинарник должен быть в src-tauri/binaries/xray".into())
    }
}
