use std::net::{IpAddr, Ipv4Addr};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::lookup_host;
use tokio::process::Command;
use tokio::sync::Mutex;

const HELPER_PATH: &str = "/usr/local/bin/frieray-tun-helper";
const SUDOERS_PATH: &str = "/etc/sudoers.d/frieray";
const TUN_DEVICE: &str = "utun99";
const HELPER_VERSION: &str = "VERSION=9_MACOS_P2P_GATEWAY";
const TUN_LOCAL_IP: &str = "198.18.0.1";
const TUN_REMOTE_IP: &str = "198.18.0.2";
const TUN_PID_PATH: &str = "/tmp/frieray-tun2socks.pid";
const TUN_LOG_PATH: &str = "/tmp/frieray-tun2socks.log";

/// Manages the tun2socks process and system routes for TUN mode
pub struct TunManager {
    original_gateway: Arc<Mutex<Option<String>>>,
    server_ip: Arc<Mutex<Option<String>>>,
}

impl TunManager {
    pub fn new() -> Self {
        Self {
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
            content.contains(HELPER_VERSION)
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
        let helper_script = format!(
            r#"#!/bin/bash
# FrieRay TUN Helper — runs with sudo NOPASSWD
# {helper_version}
set -euo pipefail

TUN2SOCKS="{tun2socks}"
DEVICE="{device}"
PID_FILE="{pid_file}"
LOG_FILE="{log_file}"

case "$1" in
    start)
        PROXY="$2"
        SERVER_IP="$3"
        GATEWAY="$4"

        # Stop stale instance before starting a new one.
        if [ -f "$PID_FILE" ]; then
            kill "$(cat "$PID_FILE")" 2>/dev/null || true
            rm -f "$PID_FILE"
        fi
        killall tun2socks 2>/dev/null || true

        # Start tun2socks in background with debug logging
        "$TUN2SOCKS" -device "$DEVICE" -proxy "$PROXY" -loglevel debug > "$LOG_FILE" 2>&1 &
        TUN_PID=$!
        echo "$TUN_PID" > "$PID_FILE" 2>/dev/null || true
        sleep 1
        kill -0 "$TUN_PID"

        # Configure point-to-point interface. macOS routes should target the remote peer.
        ifconfig "$DEVICE" "{tun_local_ip}" "{tun_remote_ip}" up

        # Setup routes: keep the VPN server outside the tunnel, send everything else into the TUN peer.
        route -n delete -host "$SERVER_IP" >/dev/null 2>&1 || true
        route -n add -host "$SERVER_IP" "$GATEWAY"
        route -n delete -net 0.0.0.0 -netmask 128.0.0.0 >/dev/null 2>&1 || true
        route -n delete -net 128.0.0.0 -netmask 128.0.0.0 >/dev/null 2>&1 || true
        route -n add -net 0.0.0.0 -netmask 128.0.0.0 "{tun_remote_ip}"
        route -n add -net 128.0.0.0 -netmask 128.0.0.0 "{tun_remote_ip}"
        sleep 1
        route -n get 1.1.1.1 | grep -Eq "gateway: {tun_remote_ip}|interface: utun"
        echo "TUN started (pid=$TUN_PID)"
        ;;

    stop)
        # Kill tun2socks
        if [ -f "$PID_FILE" ]; then
            kill "$(cat "$PID_FILE")" 2>/dev/null || true
            rm -f "$PID_FILE"
        fi
        killall tun2socks 2>/dev/null || true

        # Clean up override routes cleanly using explicit netmask
        GATEWAY="$2"
        SERVER_IP="$3"
        
        route -n delete -net 0.0.0.0 -netmask 128.0.0.0 2>/dev/null || true
        route -n delete -net 128.0.0.0 -netmask 128.0.0.0 2>/dev/null || true
        if [ -n "$SERVER_IP" ] && [ "$SERVER_IP" != "" ] && [ "$SERVER_IP" != "0.0.0.0" ]; then
            route -n delete -host "$SERVER_IP" 2>/dev/null || true
        fi
        echo "TUN stopped"
        ;;

    status)
        if pgrep -x tun2socks >/dev/null 2>&1; then
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
"#,
            tun2socks = tun2socks_path,
            helper_version = HELPER_VERSION,
            device = TUN_DEVICE,
            pid_file = TUN_PID_PATH,
            log_file = TUN_LOG_PATH,
            tun_local_ip = TUN_LOCAL_IP,
            tun_remote_ip = TUN_REMOTE_IP,
        );

        // Write helper to temp, then use osascript to install (one-time password)
        let tmp_helper = "/tmp/frieray-tun-helper";
        let tmp_sudoers = "/tmp/frieray-sudoers";

        std::fs::write(tmp_helper, &helper_script)
            .map_err(|e| format!("Write helper error: {}", e))?;

        // Create sudoers entry
        let sudoers_content = format!("{} ALL=(root) NOPASSWD: {}\n", username, HELPER_PATH);
        std::fs::write(tmp_sudoers, &sudoers_content)
            .map_err(|e| format!("Write sudoers error: {}", e))?;

        // Install with single admin password prompt
        let install_cmd = format!(
            "cp {} {} && chmod 755 {} && chown root:wheel {} && \
             cp {} {} && chmod 440 {} && chown root:wheel {}",
            tmp_helper,
            HELPER_PATH,
            HELPER_PATH,
            HELPER_PATH,
            tmp_sudoers,
            SUDOERS_PATH,
            SUDOERS_PATH,
            SUDOERS_PATH
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

        let server_route_ip = resolve_server_ipv4(vpn_server_ip).await?;

        // Get current default gateway BEFORE stopping any existing instance
        let gateway = get_default_gateway().await?;
        log::info!("Current gateway: {}", gateway);
        log::info!(
            "TUN bypass route target: {} -> {}",
            vpn_server_ip,
            server_route_ip
        );

        {
            let mut gw = self.original_gateway.lock().await;
            *gw = Some(gateway.clone());
            let mut sip = self.server_ip.lock().await;
            *sip = Some(server_route_ip.clone());
        }

        // Now that we've secured our original gateway state, we can safely stop any existing TUN
        self.stop().await.ok();

        // Start TUN via helper (no password needed)
        let proxy_url = format!("socks5://127.0.0.1:{}", socks_port);

        let result = Command::new("sudo")
            .arg(HELPER_PATH)
            .arg("start")
            .arg(&proxy_url)
            .arg(&server_route_ip)
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
        self.verify_startup(&server_route_ip).await?;

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

    async fn verify_startup(&self, server_ip: &str) -> Result<(), String> {
        if !is_tun_process_alive().await {
            let log_tail = read_tun_log_tail();
            self.stop().await.ok();
            return Err(format!(
                "tun2socks завершился сразу после старта. {}",
                log_tail
            ));
        }

        let default_route = get_route_details("1.1.1.1").await?;
        if !route_uses_tun(&default_route) {
            self.stop().await.ok();
            return Err(format!(
                "TUN маршрут не активировался: внешний трафик всё ещё идёт мимо туннеля. {}",
                compact_route_details(&default_route),
            ));
        }

        let server_route = get_route_details(server_ip).await?;
        let gateway = self
            .original_gateway
            .lock()
            .await
            .clone()
            .unwrap_or_default();
        if !gateway.is_empty() && !route_uses_gateway(&server_route, &gateway) {
            self.stop().await.ok();
            return Err(format!(
                "Маршрут до VPN-сервера {} не закрепился через исходный шлюз {}. {}",
                server_ip,
                gateway,
                compact_route_details(&server_route),
            ));
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
            log::info!(
                "Recovered gateway from networksetup/ipconfig: {}",
                fallback_gw
            );
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

async fn resolve_server_ipv4(input: &str) -> Result<String, String> {
    if input.parse::<Ipv4Addr>().is_ok() {
        return Ok(input.to_string());
    }

    if let Ok(IpAddr::V6(_)) = input.parse::<IpAddr>() {
        return Err("TUN режим сейчас поддерживает только IPv4-адрес VPN-сервера".into());
    }

    let resolved = lookup_host((input, 0))
        .await
        .map_err(|e| format!("Не удалось резолвить адрес VPN-сервера {}: {}", input, e))?;

    for addr in resolved {
        if let IpAddr::V4(ip) = addr.ip() {
            return Ok(ip.to_string());
        }
    }

    Err(format!("Для VPN-сервера {} не найден IPv4-адрес, а текущий TUN helper работает только с IPv4-маршрутами", input))
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

    let base_url = format!(
        "https://github.com/xjasonlyu/tun2socks/releases/latest/download/{}",
        filename
    );
    let mirrors = vec![
        base_url.clone(),
        format!("https://ghproxy.net/{}", base_url),
        format!("https://mirror.ghproxy.com/{}", base_url),
    ];

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))?;

    let mut success_resp = None;

    // Try all mirrors directly first
    for url in &mirrors {
        log::info!("Trying to download tun2socks from {}", url);
        if let Ok(resp) = client
            .get(url)
            .header("User-Agent", "FrieRay/0.1")
            .send()
            .await
        {
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
                    if let Ok(resp) = proxy_client
                        .get(url)
                        .header("User-Agent", "FrieRay/0.1")
                        .send()
                        .await
                    {
                        if resp.status().is_success() {
                            success_resp = Some(resp);
                            break;
                        }
                    }
                }
            }
        }
    }

    let resp = success_resp
        .ok_or_else(|| "Все попытки скачивания (прямые и через прокси) не удались".to_string())?;

    let zip_data = resp
        .bytes()
        .await
        .map_err(|e| format!("Read error: {}", e))?;

    use std::io::{Cursor, Read};
    let reader = Cursor::new(zip_data);
    let mut archive = zip::ZipArchive::new(reader).map_err(|e| format!("Zip error: {}", e))?;

    let mut binary_data = Vec::new();

    // Find the tun2socks binary in the zip (it should be the only file or named matching)
    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| format!("Zip file error: {}", e))?;
        if !file.is_dir() && file.name().contains("tun2socks") {
            file.read_to_end(&mut binary_data)
                .map_err(|e| format!("Decompress extract error: {}", e))?;
            break;
        }
    }

    if binary_data.is_empty() {
        return Err("Binary not found inside zip archive".to_string());
    }

    std::fs::write(dest, &binary_data).map_err(|e| format!("Write error: {}", e))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(dest, std::fs::Permissions::from_mode(0o755))
            .map_err(|e| format!("chmod error: {}", e))?;
    }

    log::info!(
        "tun2socks downloaded to {:?} ({} bytes)",
        dest,
        binary_data.len()
    );
    Ok(())
}

async fn is_tun_process_alive() -> bool {
    Command::new("pgrep")
        .arg("-x")
        .arg("tun2socks")
        .output()
        .await
        .map(|out| out.status.success())
        .unwrap_or(false)
}

async fn get_route_details(target: &str) -> Result<String, String> {
    let output = Command::new("route")
        .arg("-n")
        .arg("get")
        .arg(target)
        .output()
        .await
        .map_err(|e| format!("Не удалось прочитать маршрут для {}: {}", target, e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(format!(
            "route get {} завершился ошибкой: {}",
            target, stderr
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn route_uses_interface(route_output: &str, interface: &str) -> bool {
    route_output
        .lines()
        .any(|line| line.trim() == format!("interface: {}", interface))
}

fn route_uses_tun(route_output: &str) -> bool {
    route_uses_gateway(route_output, TUN_REMOTE_IP)
        || route_uses_interface(route_output, TUN_DEVICE)
        || route_output
            .lines()
            .any(|line| line.trim().starts_with("interface: utun"))
}

fn route_uses_gateway(route_output: &str, gateway: &str) -> bool {
    route_output
        .lines()
        .any(|line| line.trim() == format!("gateway: {}", gateway))
}

fn compact_route_details(route_output: &str) -> String {
    let summary = route_output
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.starts_with("gateway:") || line.starts_with("interface:") {
                Some(line.to_string())
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .join(", ");

    if summary.is_empty() {
        "route get не вернул gateway/interface".into()
    } else {
        format!("Найдено: {}", summary)
    }
}

fn read_tun_log_tail() -> String {
    let Ok(content) = std::fs::read_to_string(TUN_LOG_PATH) else {
        return "Лог tun2socks пуст или недоступен".into();
    };

    let tail = content
        .lines()
        .rev()
        .take(8)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join(" | ");

    if tail.is_empty() {
        "Лог tun2socks пуст или недоступен".into()
    } else {
        format!("Последние строки tun2socks: {}", tail)
    }
}
