use tauri::State;
use crate::AppState;
use crate::utils::log_buffer::LogEntry;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct TrafficStats {
    pub uplink: u64,     // total bytes uploaded
    pub downlink: u64,   // total bytes downloaded
    pub up_speed: f64,   // bytes/sec
    pub down_speed: f64, // bytes/sec
}

static LAST_STATS: std::sync::OnceLock<tokio::sync::Mutex<(u64, u64, std::time::Instant)>> = std::sync::OnceLock::new();

fn get_last_stats() -> &'static tokio::sync::Mutex<(u64, u64, std::time::Instant)> {
    LAST_STATS.get_or_init(|| tokio::sync::Mutex::new((0, 0, std::time::Instant::now())))
}

#[tauri::command]
pub async fn get_logs(state: State<'_, AppState>) -> Result<Vec<LogEntry>, String> {
    Ok(state.logs.get_all().await)
}

#[tauri::command]
pub async fn clear_logs(state: State<'_, AppState>) -> Result<(), String> {
    state.logs.clear().await;
    Ok(())
}

/// Get real-time traffic stats from xray API
#[tauri::command]
pub async fn get_traffic_stats(state: State<'_, AppState>) -> Result<TrafficStats, String> {
    if !state.xray.is_running().await {
        return Ok(TrafficStats { uplink: 0, downlink: 0, up_speed: 0.0, down_speed: 0.0 });
    }

    // Query xray stats via the xray binary's CLI API
    let xray_bin = find_xray_bin();

    let uplink = query_stat(&xray_bin, "outbound>>>proxy>>>traffic>>>uplink").await.unwrap_or(0);
    let downlink = query_stat(&xray_bin, "outbound>>>proxy>>>traffic>>>downlink").await.unwrap_or(0);

    // Calculate speed (bytes/sec)
    let mut last = get_last_stats().lock().await;
    let now = std::time::Instant::now();
    let elapsed = now.duration_since(last.2).as_secs_f64();

    let (up_speed, down_speed) = if elapsed > 0.1 && last.0 > 0 {
        let up_diff = if uplink > last.0 { uplink - last.0 } else { 0 };
        let down_diff = if downlink > last.1 { downlink - last.1 } else { 0 };
        (up_diff as f64 / elapsed, down_diff as f64 / elapsed)
    } else {
        (0.0, 0.0)
    };

    *last = (uplink, downlink, now);

    Ok(TrafficStats {
        uplink,
        downlink,
        up_speed,
        down_speed,
    })
}

/// Speed test — download through the local SOCKS proxy
#[tauri::command]
pub async fn speed_test(state: State<'_, AppState>) -> Result<f64, String> {
    if !state.xray.is_running().await {
        return Err("Нет подключения".into());
    }

    let settings = state.settings.lock().await;
    let socks_port = settings.proxy.socks_port;
    drop(settings);

    state.logs.add("info", "Запуск теста скорости...").await;

    let proxy = reqwest::Proxy::all(format!("socks5://127.0.0.1:{}", socks_port))
        .map_err(|e| format!("Proxy error: {}", e))?;

    let client = reqwest::Client::builder()
        .proxy(proxy)
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("Client error: {}", e))?;

    // Download ~1MB to measure speed
    let start = std::time::Instant::now();
    let resp = client
        .get("https://speed.cloudflare.com/__down?bytes=1000000")
        .send()
        .await
        .map_err(|e| format!("Download error: {}", e))?;

    let bytes = resp
        .bytes()
        .await
        .map_err(|e| format!("Read error: {}", e))?;

    let elapsed = start.elapsed().as_secs_f64();
    let mbps = (bytes.len() as f64 * 8.0) / (elapsed * 1_000_000.0);

    state.logs.add("success", &format!("Скорость: {:.1} Мбит/с ({} байт за {:.1}с)", mbps, bytes.len(), elapsed)).await;

    Ok(mbps)
}

/// Query a single stat value from xray API
async fn query_stat(xray_bin: &str, stat_name: &str) -> Option<u64> {
    let output = tokio::process::Command::new(xray_bin)
        .arg("api")
        .arg("statsquery")
        .arg("--server=127.0.0.1:10085")
        .arg(format!("-name={}", stat_name))
        .output()
        .await
        .ok()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Parse "stat: { name: "...", value: 12345 }" format
    for line in stdout.lines() {
        let line = line.trim();
        if line.starts_with("\"value\"") || line.contains("value") {
            // Extract number from line like: "value": "12345" or value: 12345
            let digits: String = line.chars().filter(|c| c.is_ascii_digit()).collect();
            if !digits.is_empty() {
                return digits.parse().ok();
            }
        }
    }

    // If parsing fails, try to get any number from output
    let all_digits: String = stdout.chars().filter(|c| c.is_ascii_digit()).collect();
    if !all_digits.is_empty() && all_digits != "10085" {
        return all_digits.parse().ok();
    }

    Some(0)
}

fn find_xray_bin() -> String {
    if let Ok(exe) = std::env::current_exe() {
        let dir = exe.parent().unwrap_or(std::path::Path::new("."));
        let path = dir.join("xray");
        if path.exists() {
            return path.to_string_lossy().to_string();
        }
    }

    let project = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("binaries/xray");
    if project.exists() {
        return project.to_string_lossy().to_string();
    }

    "xray".to_string()
}
