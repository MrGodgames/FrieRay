use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

const HELPER_PATH: &str = "/usr/local/bin/frieray-tun-helper";
const SUDOERS_PATH: &str = "/etc/sudoers.d/frieray";

/// Manages the tun2socks process and system routes for TUN mode
pub struct TunManager {
    process: Arc<Mutex<Option<Child>>>,
    original_gateway: Arc<Mutex<Option<String>>>,
    server_ip: Arc<Mutex<Option<String>>>,
}

impl TunManager {
    pub fn new() -> Self {
        Self {
            process: Arc::new(Mutex::new(None)),
            original_gateway: Arc::new(Mutex::new(None)),
            server_ip: Arc::new(Mutex::new(None)),
        }
    }

    /// Check if the privileged helper is installed and up-to-date
    pub fn is_helper_installed(&self) -> bool {
        if !PathBuf::from(HELPER_PATH).exists() || !PathBuf::from(SUDOERS_PATH).exists() {
            return false;
        }
        // Check if helper has the latest safe routing logic
        if let Ok(content) = std::fs::read_to_string(HELPER_PATH) {
            content.contains("VERSION=7_MACOS_QUOTED_P2P")
        } else {
            false
        }
    }

    /// Install the privileged helper — asks for password ONCE
    pub async fn install_helper(&self) -> Result<(), String> {
        log::info!("Installing TUN helper (one-time password required)...");

        let tun2socks_bin = self.find_or_download_tun2socks().await?;
        let tun2socks_path = tun2socks_bin.to_string_lossy().to_string();

        let username = whoami().await;

        // Create helper script content
        let helper_script = format!(r#"#!/bin/bash
# FrieRay TUN Helper — runs with sudo NOPASSWD
# VERSION=7_MACOS_QUOTED_P2P
set -e

TUN2SOCKS="{tun2socks}"
DEVICE="utun99"

case "$1" in
    start)
        PROXY="$2"
        SERVER_IP="$3"
        GATEWAY="$4"

        # Start tun2socks in background with debug logging
        "$TUN2SOCKS" -device $DEVICE -proxy "$PROXY" -loglevel debug > /tmp/frieray-tun2socks.log 2>&1 &
        echo $! > /tmp/frieray-tun2socks.pid
        sleep 1

        # Configure point-to-point interface: Local 198.18.0.1, Remote 198.18.0.2
        ifconfig $DEVICE 198.18.0.1 198.18.0.2 up 2>/dev/null || true

        # Setup standard macOS TUN routes pointing to the REMOTE peer (198.18.0.2)
        route add "$SERVER_IP" "$GATEWAY" 2>/dev/null || true
        route add -net 0.0.0.0/1 -interface $DEVICE 2>/dev/null || true
        route add -net 128.0.0.0/1 -interface $DEVICE 2>/dev/null || true
        echo "TUN started"
        ;;

    stop)
        # Kill tun2socks
        if [ -f /tmp/frieray-tun2socks.pid ]; then
            kill $(cat /tmp/frieray-tun2socks.pid) 2>/dev/null || true
            rm -f /tmp/frieray-tun2socks.pid
        fi
        killall tun2socks 2>/dev/null || true

        # Clean up override routes cleanly using explicit netmask
        GATEWAY="$2"
        SERVER_IP="$3"
        
        route delete -net 0.0.0.0 -netmask 128.0.0.0 -interface $DEVICE 2>/dev/null || true
        route delete -net 128.0.0.0 -netmask 128.0.0.0 -interface $DEVICE 2>/dev/null || true
        if [ -n "$SERVER_IP" ] && [ "$SERVER_IP" != "" ] && [ "$SERVER_IP" != "0.0.0.0" ]; then
            route delete "$SERVER_IP" 2>/dev/null || true
        fi
        echo "TUN stopped"
        ;;

    status)
        if [ -f /tmp/frieray-tun2socks.pid ] && kill -0 $(cat /tmp/frieray-tun2socks.pid) 2>/dev/null; then
            echo "running"
        else
            echo "stopped"
        fi
        ;;

    *)
        echo "Usage: $0 {{start|stop|status}}"
        exit 1
        ;;
esac
"#, tun2socks = tun2socks_path);

        // Write helper to temp, then use osascript to install (one-time password)
        let tmp_helper = "/tmp/frieray-tun-helper";
        let tmp_sudoers = "/tmp/frieray-sudoers";

        std::fs::write(tmp_helper, &helper_script)
            .map_err(|e| format!("Write helper error: {}", e))?;

        // Create sudoers entry
        let sudoers_content = format!(
            "{} ALL=(root) NOPASSWD: {}\n",
            username, HELPER_PATH
        );
        std::fs::write(tmp_sudoers, &sudoers_content)
            .map_err(|e| format!("Write sudoers error: {}", e))?;

        // Install with single admin password prompt
        let install_cmd = format!(
            "cp {} {} && chmod 755 {} && chown root:wheel {} && \
             cp {} {} && chmod 440 {} && chown root:wheel {}",
            tmp_helper, HELPER_PATH, HELPER_PATH, HELPER_PATH,
            tmp_sudoers, SUDOERS_PATH, SUDOERS_PATH, SUDOERS_PATH
        );

        let result = Command::new("osascript")
            .arg("-e")
            .arg(format!(
                r#"do shell script "{}" with administrator privileges"#,
                install_cmd
            ))
            .output()
            .await
            .map_err(|e| format!("Install error: {}", e))?;

        if !result.status.success() {
            let stderr = String::from_utf8_lossy(&result.stderr);
            return Err(format!("Установка TUN helper не удалась: {}", stderr));
        }

        // Cleanup temp files
        std::fs::remove_file(tmp_helper).ok();
        std::fs::remove_file(tmp_sudoers).ok();

        log::info!("TUN helper installed at {} (passwordless)", HELPER_PATH);
        Ok(())
    }

    /// Start TUN mode — passwordless if helper is installed
    pub async fn start(&self, socks_port: u16, vpn_server_ip: &str) -> Result<(), String> {
        // Install helper if not yet installed (asks password once)
        if !self.is_helper_installed() {
            self.install_helper().await?;
        }

        // Get current default gateway BEFORE stopping any existing instance
        let gateway = get_default_gateway().await?;
        log::info!("Current gateway: {}", gateway);

        {
            let mut gw = self.original_gateway.lock().await;
            *gw = Some(gateway.clone());
            let mut sip = self.server_ip.lock().await;
            *sip = Some(vpn_server_ip.to_string());
        }

        // Now that we've secured our original gateway state, we can safely stop any existing TUN
        self.stop().await.ok();

        // Start TUN via helper (no password needed)
        let proxy_url = format!("socks5://127.0.0.1:{}", socks_port);

        let result = Command::new("sudo")
            .arg(HELPER_PATH)
            .arg("start")
            .arg(&proxy_url)
            .arg(vpn_server_ip)
            .arg(&gateway)
            .output()
            .await
            .map_err(|e| format!("TUN start error: {}", e))?;

        if !result.status.success() {
            let stderr = String::from_utf8_lossy(&result.stderr);
            return Err(format!("TUN start failed: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&result.stdout);
        log::info!("TUN helper: {}", stdout.trim());

        Ok(())
    }

    /// Stop TUN mode — passwordless
    pub async fn stop(&self) -> Result<(), String> {
        if !PathBuf::from(HELPER_PATH).exists() {
            return Ok(());
        }

        let gateway = self.original_gateway.lock().await.clone();
        let server_ip = self.server_ip.lock().await.clone();

        let gw = gateway.as_deref().unwrap_or("");
        let sip = server_ip.as_deref().unwrap_or("");

        let result = Command::new("sudo")
            .arg(HELPER_PATH)
            .arg("stop")
            .arg(gw)
            .arg(sip)
            .output()
            .await;

        match result {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                log::info!("TUN helper stop: {}", stdout.trim());
            }
            Err(e) => {
                log::warn!("TUN stop error: {}", e);
            }
        }

        Ok(())
    }

    /// Find tun2socks binary or download it
    async fn find_or_download_tun2socks(&self) -> Result<PathBuf, String> {
        let candidates = vec![
            config_dir().join("tun2socks"),
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("binaries/tun2socks"),
        ];

        for path in &candidates {
            if path.exists() {
                return Ok(path.clone());
            }
        }

        log::info!("tun2socks not found, downloading...");
        let dest = config_dir().join("tun2socks");
        download_tun2socks(&dest).await?;
        Ok(dest)
    }
}

/// Get default gateway IP, robust against TUN hijacking and app crashes
async fn get_default_gateway() -> Result<String, String> {
    // 1. Try standard routing table
    let output = Command::new("route")
        .arg("-n")
        .arg("get")
        .arg("default")
        .output()
        .await
        .map_err(|e| format!("Cannot get gateway: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut gw = String::new();
    for line in stdout.lines() {
        let line = line.trim();
        if line.starts_with("gateway:") {
            gw = line.replace("gateway:", "").trim().to_string();
            break;
        }
    }

    // If it's a real external gateway, cache it and return
    if !gw.is_empty() && gw != "198.18.0.1" && gw != "127.0.0.1" && gw != "0.0.0.0" {
        std::fs::write("/tmp/frieray-gateway.txt", &gw).ok();
        return Ok(gw);
    }

    // 2. Routing table broken or hijacked by TUN, try reading from cache
    if let Ok(cached) = std::fs::read_to_string("/tmp/frieray-gateway.txt") {
        let cached = cached.trim().to_string();
        if !cached.is_empty() && cached != "198.18.0.1" {
            log::info!("Recovered gateway from cache: {}", cached);
            return Ok(cached);
        }
    }

    // 3. Last resort fallback: poll DHCP lease data via ipconfig
    let fallback = Command::new("sh")
        .arg("-c")
        .arg("ipconfig getoption en0 router || ipconfig getoption en1 router")
        .output()
        .await;
        
    if let Ok(out) = fallback {
        let fallback_gw = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !fallback_gw.is_empty() {
            log::info!("Recovered gateway from networksetup/ipconfig: {}", fallback_gw);
            std::fs::write("/tmp/frieray-gateway.txt", &fallback_gw).ok();
            return Ok(fallback_gw);
        }
    }

    Err("Не удалось определить шлюз по умолчанию (даже резервными методами)".into())
}

async fn whoami() -> String {
    Command::new("whoami")
        .output()
        .await
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "nobody".into())
}

fn config_dir() -> PathBuf {
    let dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("frieray");
    std::fs::create_dir_all(&dir).ok();
    dir
}

/// Download tun2socks binary
async fn download_tun2socks(dest: &PathBuf) -> Result<(), String> {
    let filename = if cfg!(target_arch = "aarch64") {
        "tun2socks-darwin-arm64.zip"
    } else {
        "tun2socks-darwin-amd64.zip"
    };

    let base_url = format!("https://github.com/xjasonlyu/tun2socks/releases/latest/download/{}", filename);
    let mirrors = vec![
        base_url.clone(),
        format!("https://ghproxy.net/{}", base_url),
        format!("https://mirror.ghproxy.com/{}", base_url),
    ];

    let mut client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))?;

    let mut success_resp = None;

    // Try all mirrors directly first
    for url in &mirrors {
        log::info!("Trying to download tun2socks from {}", url);
        if let Ok(resp) = client.get(url).header("User-Agent", "FrieRay/0.1").send().await {
            if resp.status().is_success() {
                success_resp = Some(resp);
                break;
            }
        }
    }

    // If all direct downloads failed, try with local proxy (only works if Xray is currently running)
    if success_resp.is_none() {
        log::info!("Direct downloads failed, trying with local proxy...");
        let settings = crate::utils::storage::load_app_settings();
        let proxy_url = format!("socks5h://127.0.0.1:{}", settings.proxy.socks_port);
        
        if let Ok(proxy) = reqwest::Proxy::all(&proxy_url) {
            if let Ok(proxy_client) = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .redirect(reqwest::redirect::Policy::limited(10))
                .proxy(proxy)
                .build() 
            {
                for url in &mirrors {
                    if let Ok(resp) = proxy_client.get(url).header("User-Agent", "FrieRay/0.1").send().await {
                        if resp.status().is_success() {
                            success_resp = Some(resp);
                            break;
                        }
                    }
                }
            }
        }
    }

    let resp = success_resp.ok_or_else(|| "Все попытки скачивания (прямые и через прокси) не удались".to_string())?;

    let zip_data = resp.bytes().await
        .map_err(|e| format!("Read error: {}", e))?;

    use std::io::{Read, Cursor};
    let reader = Cursor::new(zip_data);
    let mut archive = zip::ZipArchive::new(reader)
        .map_err(|e| format!("Zip error: {}", e))?;

    let mut binary_data = Vec::new();
    
    // Find the tun2socks binary in the zip (it should be the only file or named matching)
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| format!("Zip file error: {}", e))?;
        if !file.is_dir() && file.name().contains("tun2socks") {
            file.read_to_end(&mut binary_data)
                .map_err(|e| format!("Decompress extract error: {}", e))?;
            break;
        }
    }

    if binary_data.is_empty() {
        return Err("Binary not found inside zip archive".to_string());
    }

    std::fs::write(dest, &binary_data)
        .map_err(|e| format!("Write error: {}", e))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(dest, std::fs::Permissions::from_mode(0o755))
            .map_err(|e| format!("chmod error: {}", e))?;
    }

    log::info!("tun2socks downloaded to {:?} ({} bytes)", dest, binary_data.len());
    Ok(())
}
