use crate::core::config::generate_xray_config;
use crate::core::xray::XrayManager;
use crate::models::server::{Server, Subscription};
use crate::models::settings::AppSettings;
use crate::utils::storage;
use crate::utils::subscription::{fetch_subscription, parse_subscription_content};
use crate::utils::vless;
use crate::AppState;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use tauri::State;
use tokio::process::Command;
use tokio::sync::Semaphore;

const SERVER_SPEED_TEST_TARGETS: &[(&str, &str)] = &[
    ("Selectel", "https://speedtest.selectel.ru/100MB"),
    ("Yandex Mirror", "https://mirror.yandex.ru/debian/ls-lR.gz"),
];
const SERVER_SPEED_TEST_BYTES: u64 = 256 * 1024;
const SERVER_SPEED_TEST_TIMEOUT_SECS: u64 = 6;
const SERVER_SPEED_TEST_CONCURRENCY: usize = 10;
const PING_ATTEMPTS: usize = 3;
const PING_TIMEOUT_MS: u64 = 2500;

#[tauri::command]
pub async fn add_subscription(
    name: String,
    url: String,
    state: State<'_, AppState>,
) -> Result<Subscription, String> {
    let sub = Subscription {
        id: uuid::Uuid::new_v4().to_string(),
        name: if name.is_empty() {
            "Подписка".into()
        } else {
            name
        },
        url,
        last_update: None,
        server_count: 0,
    };

    let mut subs = state.subscriptions.lock().await;
    subs.push(sub.clone());
    storage::save_subscriptions(&subs)?;

    log::info!("Subscription added: {}", sub.name);
    Ok(sub)
}

#[tauri::command]
pub async fn remove_subscription(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let mut subs = state.subscriptions.lock().await;
    subs.retain(|s| s.id != id);
    storage::save_subscriptions(&subs)?;

    let mut servers = state.servers.lock().await;
    servers.retain(|s| s.subscription_id.as_deref() != Some(&id));
    storage::save_servers(&servers)?;

    Ok(())
}

#[tauri::command]
pub async fn update_subscriptions(state: State<'_, AppState>) -> Result<Vec<Server>, String> {
    let subs = state.subscriptions.lock().await.clone();

    if subs.is_empty() {
        return Err("Нет подписок. Добавьте подписку сначала.".into());
    }

    let mut all_servers = Vec::new();
    let mut errors = Vec::new();

    for sub in &subs {
        match fetch_subscription(sub).await {
            Ok(servers) => {
                log::info!("Fetched {} servers from '{}'", servers.len(), sub.name);
                if servers.is_empty() {
                    errors.push(format!("'{}': 0 серверов", sub.name));
                }
                all_servers.extend(servers);
            }
            Err(e) => {
                log::error!("Failed to fetch '{}': {}", sub.name, e);
                errors.push(format!("'{}': {}", sub.name, e));
            }
        }
    }

    // Update server list
    let mut servers = state.servers.lock().await;
    servers.retain(|s| s.subscription_id.is_none());
    servers.extend(all_servers.clone());
    storage::save_servers(&servers)?;

    // Update subscription metadata
    let mut subs_lock = state.subscriptions.lock().await;
    for sub in subs_lock.iter_mut() {
        sub.server_count = servers
            .iter()
            .filter(|s| s.subscription_id.as_deref() == Some(&sub.id))
            .count();
        sub.last_update = Some(chrono_now());
    }
    storage::save_subscriptions(&subs_lock)?;

    if servers.is_empty() && !errors.is_empty() {
        return Err(format!("Ошибки: {}", errors.join("; ")));
    }

    Ok(servers.clone())
}

#[tauri::command]
pub async fn get_servers(state: State<'_, AppState>) -> Result<Vec<Server>, String> {
    let servers = state.servers.lock().await;
    Ok(servers.clone())
}

#[tauri::command]
pub async fn get_subscriptions(state: State<'_, AppState>) -> Result<Vec<Subscription>, String> {
    let subs = state.subscriptions.lock().await;
    Ok(subs.clone())
}

#[tauri::command]
pub fn parse_link(link: String) -> Result<Server, String> {
    let link = link.trim();
    if link.starts_with("vless://") {
        vless::parse_vless_url(link)
    } else if link.starts_with("vmess://")
        || link.starts_with("trojan://")
        || link.starts_with("ss://")
    {
        let servers = parse_subscription_content(link, "")?;
        servers
            .into_iter()
            .next()
            .ok_or("Не удалось распарсить ссылку".into())
    } else {
        Err(format!(
            "Неподдерживаемый протокол: {}",
            &link[..link.len().min(30)]
        ))
    }
}

/// Set the active server for connection
#[tauri::command]
pub async fn set_active_server(
    server_id: String,
    state: State<'_, AppState>,
) -> Result<Server, String> {
    let servers = state.servers.lock().await;
    let server = servers
        .iter()
        .find(|s| s.id == server_id)
        .cloned()
        .ok_or("Сервер не найден")?;

    let mut active = state.active_server.lock().await;
    *active = Some(server.clone());
    storage::save_active_server_id(&server.id)?;

    log::info!("Active server set: {}", server.name);
    Ok(server)
}

#[tauri::command]
pub async fn get_active_server(state: State<'_, AppState>) -> Result<Option<Server>, String> {
    let active = state.active_server.lock().await;
    Ok(active.clone())
}

/// Ping a single server via TCP connect
#[tauri::command]
pub async fn ping_server(address: String, port: u16) -> Result<u32, String> {
    let probe = measure_server_ping(&address, port).await?;
    probe.ping.ok_or_else(|| "Сервер недоступен".into())
}

/// Ping all servers in parallel and return sorted list
#[tauri::command]
pub async fn ping_all_servers(state: State<'_, AppState>) -> Result<Vec<Server>, String> {
    let servers = state.servers.lock().await.clone();
    let mut join_set = tokio::task::JoinSet::new();

    for server in &servers {
        let server_id = server.id.clone();
        let addr = server.address.clone();
        let port = server.port;
        join_set.spawn(async move { (server_id, measure_server_ping(&addr, port).await.ok()) });
    }

    while let Some(result) = join_set.join_next().await {
        if let Ok((server_id, probe)) = result {
            let mut state_servers = state.servers.lock().await;
            if let Some(server) = state_servers.iter_mut().find(|s| s.id == server_id) {
                match probe {
                    Some(probe) => {
                        server.ping = probe.ping;
                        server.reachable = Some(probe.reachable);
                        if !probe.reachable {
                            server.speed_mbps = None;
                        }
                    }
                    None => {
                        server.ping = None;
                        server.reachable = Some(false);
                        server.speed_mbps = None;
                    }
                }
            }
        }
    }

    let mut servers = state.servers.lock().await.clone();
    servers.sort_by(|a, b| match (a.ping, b.ping) {
        (Some(pa), Some(pb)) => pa.cmp(&pb),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    });

    {
        let mut state_servers = state.servers.lock().await;
        *state_servers = servers.clone();
    }
    storage::save_servers(&servers)?;

    Ok(servers)
}

/// Speed test all servers using temporary isolated Xray instances.
#[tauri::command]
pub async fn speed_test_all_servers(state: State<'_, AppState>) -> Result<Vec<Server>, String> {
    let settings = state.settings.lock().await.clone();
    let current_servers = state.servers.lock().await.clone();

    if current_servers.is_empty() {
        return Err("Нет серверов для теста скорости".into());
    }

    let mut reachable_servers = Vec::new();
    let mut skipped_servers = Vec::new();
    let has_ping_data = current_servers.iter().any(|server| server.ping.is_some());

    for server in current_servers {
        if has_ping_data && server.ping.is_none() {
            skipped_servers.push(server.id);
        } else {
            reachable_servers.push(server);
        }
    }

    if reachable_servers.is_empty() {
        return Err("Нет доступных серверов для теста скорости. Сначала запусти пинг.".into());
    }

    state
        .logs
        .add(
            "info",
            &format!(
                "Запуск теста скорости для {} серверов...",
                reachable_servers.len()
            ),
        )
        .await;

    let semaphore = std::sync::Arc::new(Semaphore::new(SERVER_SPEED_TEST_CONCURRENCY));
    let mut join_set = tokio::task::JoinSet::new();

    for server in reachable_servers {
        let permit_pool = semaphore.clone();
        let settings = settings.clone();
        join_set.spawn(async move {
            let _permit = permit_pool.acquire_owned().await.ok();
            let speed = speed_test_server(&server, &settings).await.ok();
            (server.id, speed)
        });
    }

    let mut speed_by_id = HashMap::new();
    while let Some(result) = join_set.join_next().await {
        if let Ok((server_id, speed)) = result {
            let lookup_id = server_id.clone();
            speed_by_id.insert(server_id, speed);
            let mut state_servers = state.servers.lock().await;
            if let Some(server) = state_servers
                .iter_mut()
                .find(|server| server.id == lookup_id)
            {
                server.speed_mbps = speed;
            }
        }
    }

    let mut servers = state.servers.lock().await.clone();
    for server in &mut servers {
        if skipped_servers.iter().any(|id| id == &server.id) {
            server.speed_mbps = None;
            continue;
        }
        server.speed_mbps = speed_by_id.get(&server.id).copied().flatten();
    }

    servers.sort_by(|a, b| match (a.speed_mbps, b.speed_mbps) {
        (Some(sa), Some(sb)) => sb
            .partial_cmp(&sa)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| match (a.ping, b.ping) {
                (Some(pa), Some(pb)) => pa.cmp(&pb),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            }),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => match (a.ping, b.ping) {
            (Some(pa), Some(pb)) => pa.cmp(&pb),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        },
    });

    {
        let mut state_servers = state.servers.lock().await;
        *state_servers = servers.clone();
    }
    storage::save_servers(&servers)?;

    let success_count = servers.iter().filter(|s| s.speed_mbps.is_some()).count();
    state
        .logs
        .add(
            "success",
            &format!(
                "Тест скорости завершён: {} из {} серверов ответили",
                success_count,
                speed_by_id.len()
            ),
        )
        .await;

    Ok(servers)
}

fn chrono_now() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    let hours = (secs / 3600) % 24;
    let mins = (secs / 60) % 60;
    format!("{:02}:{:02}", hours, mins)
}

async fn speed_test_server(server: &Server, settings: &AppSettings) -> Result<f64, String> {
    let socks_port = reserve_local_port()?;
    let http_port = reserve_local_port()?;
    let api_port = reserve_local_port()?;

    let mut temp_settings = settings.clone();
    temp_settings.proxy.socks_port = socks_port;
    temp_settings.proxy.http_port = http_port;

    let config = generate_xray_config(server, &temp_settings, api_port);
    let config_path = temp_config_path(&server.id);
    let config_json =
        serde_json::to_string_pretty(&config).map_err(|e| format!("Config error: {}", e))?;
    std::fs::write(&config_path, config_json).map_err(|e| format!("Write error: {}", e))?;

    let xray_bin = XrayManager::new().find_xray_binary()?;
    let mut child = Command::new(&xray_bin)
        .arg("run")
        .arg("-config")
        .arg(&config_path)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| format!("Failed to start temp xray: {}", e))?;

    let result = async {
        tokio::time::sleep(std::time::Duration::from_millis(700)).await;

        if let Some(status) = child
            .try_wait()
            .map_err(|e| format!("Xray status error: {}", e))?
        {
            let stderr_msg = read_child_stderr(&mut child).await;
            return Err(if stderr_msg.is_empty() {
                format!("Xray exited early with status {}", status)
            } else {
                format!("Xray exited early: {}", stderr_msg)
            });
        }

        let proxy = reqwest::Proxy::all(format!("socks5h://127.0.0.1:{}", socks_port))
            .map_err(|e| format!("Proxy error: {}", e))?;

        let client = reqwest::Client::builder()
            .proxy(proxy)
            .timeout(std::time::Duration::from_secs(
                SERVER_SPEED_TEST_TIMEOUT_SECS,
            ))
            .build()
            .map_err(|e| format!("Client error: {}", e))?;

        let mut errors = Vec::new();
        for (name, url) in SERVER_SPEED_TEST_TARGETS {
            match run_server_speed_test(&client, url).await {
                Ok(mbps) => return Ok(mbps),
                Err(e) => errors.push(format!("{}: {}", name, e)),
            }
        }

        Err(errors.join(" | "))
    }
    .await;

    let _ = child.kill().await;
    let _ = tokio::time::timeout(std::time::Duration::from_secs(2), child.wait()).await;
    let _ = std::fs::remove_file(&config_path);

    result
}

fn reserve_local_port() -> Result<u16, String> {
    let listener = std::net::TcpListener::bind("127.0.0.1:0")
        .map_err(|e| format!("Port bind error: {}", e))?;
    let port = listener
        .local_addr()
        .map_err(|e| format!("Local addr error: {}", e))?
        .port();
    drop(listener);
    Ok(port)
}

fn temp_config_path(server_id: &str) -> PathBuf {
    std::env::temp_dir().join(format!("frieray-speedtest-{}.json", server_id))
}

async fn read_child_stderr(child: &mut tokio::process::Child) -> String {
    if let Some(mut stderr) = child.stderr.take() {
        use tokio::io::AsyncReadExt;
        let mut buf = vec![0u8; 4096];
        match tokio::time::timeout(std::time::Duration::from_secs(1), stderr.read(&mut buf)).await {
            Ok(Ok(n)) if n > 0 => String::from_utf8_lossy(&buf[..n]).trim().to_string(),
            _ => String::new(),
        }
    } else {
        String::new()
    }
}

async fn run_server_speed_test(client: &reqwest::Client, url: &str) -> Result<f64, String> {
    let start = std::time::Instant::now();
    let response = client
        .get(url)
        .header(
            reqwest::header::RANGE,
            format!("bytes=0-{}", SERVER_SPEED_TEST_BYTES - 1),
        )
        .send()
        .await
        .map_err(|e| format!("request error for {}: {}", url, e))?
        .error_for_status()
        .map_err(|e| format!("HTTP error for {}: {}", url, e))?;

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("read error for {}: {}", url, e))?;

    if bytes.is_empty() {
        return Err(format!("{} returned empty response", url));
    }

    let elapsed = start.elapsed().as_secs_f64();
    if elapsed <= 0.0 {
        return Err(format!("{} returned too quickly to measure", url));
    }

    Ok((bytes.len() as f64 * 8.0) / (elapsed * 1_000_000.0))
}

#[derive(Debug, Clone, Copy)]
struct PingProbe {
    ping: Option<u32>,
    reachable: bool,
}

async fn measure_server_ping(address: &str, port: u16) -> Result<PingProbe, String> {
    use tokio::net::TcpStream;
    use tokio::time::{timeout, Duration};

    let socket_addr = tokio::net::lookup_host((address, port))
        .await
        .map_err(|e| format!("DNS error: {}", e))?
        .next()
        .ok_or("Cannot resolve address")?;

    let mut samples = Vec::new();

    for _ in 0..PING_ATTEMPTS {
        let start = std::time::Instant::now();
        if let Ok(Ok(_)) = timeout(
            Duration::from_millis(PING_TIMEOUT_MS),
            TcpStream::connect(socket_addr),
        )
        .await
        {
            samples.push(start.elapsed().as_millis() as u32);
        }
    }

    if samples.is_empty() {
        return Ok(PingProbe {
            ping: None,
            reachable: false,
        });
    }

    samples.sort_unstable();
    Ok(PingProbe {
        ping: Some(samples[samples.len() / 2]),
        reachable: true,
    })
}
