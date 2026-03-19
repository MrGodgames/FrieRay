use tauri::State;
use crate::AppState;
use crate::models::server::{Server, Subscription};
use crate::utils::subscription::{fetch_subscription, parse_subscription_content};
use crate::utils::storage;
use crate::utils::vless;

#[tauri::command]
pub async fn add_subscription(
    name: String,
    url: String,
    state: State<'_, AppState>,
) -> Result<Subscription, String> {
    let sub = Subscription {
        id: uuid::Uuid::new_v4().to_string(),
        name: if name.is_empty() { "Подписка".into() } else { name },
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
pub async fn remove_subscription(
    id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
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
    } else if link.starts_with("vmess://") || link.starts_with("trojan://") || link.starts_with("ss://") {
        let servers = parse_subscription_content(link, "")?;
        servers.into_iter().next().ok_or("Не удалось распарсить ссылку".into())
    } else {
        Err(format!("Неподдерживаемый протокол: {}", &link[..link.len().min(30)]))
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
    use std::net::ToSocketAddrs;
    use tokio::net::TcpStream;
    use tokio::time::{timeout, Duration};

    let addr_str = format!("{}:{}", address, port);
    let socket_addr = addr_str
        .to_socket_addrs()
        .map_err(|e| format!("DNS error: {}", e))?
        .next()
        .ok_or("Cannot resolve address")?;

    let start = std::time::Instant::now();
    match timeout(Duration::from_secs(5), TcpStream::connect(socket_addr)).await {
        Ok(Ok(_)) => Ok(start.elapsed().as_millis() as u32),
        Ok(Err(e)) => Err(format!("Connection failed: {}", e)),
        Err(_) => Err("Timeout (5s)".into()),
    }
}

/// Ping all servers in parallel and return sorted list
#[tauri::command]
pub async fn ping_all_servers(state: State<'_, AppState>) -> Result<Vec<Server>, String> {
    use tokio::time::{timeout, Duration};

    let mut servers = state.servers.lock().await.clone();

    let mut handles = Vec::new();
    for server in &servers {
        let addr = server.address.clone();
        let port = server.port;
        handles.push(tokio::spawn(async move {
            use std::net::ToSocketAddrs;
            use tokio::net::TcpStream;

            let addr_str = format!("{}:{}", addr, port);
            if let Ok(Some(socket_addr)) = addr_str.to_socket_addrs().map(|mut a| a.next()) {
                let start = std::time::Instant::now();
                match timeout(Duration::from_secs(5), TcpStream::connect(socket_addr)).await {
                    Ok(Ok(_)) => Some(start.elapsed().as_millis() as u32),
                    _ => None,
                }
            } else {
                None
            }
        }));
    }

    for (i, handle) in handles.into_iter().enumerate() {
        if let Ok(ping) = handle.await {
            servers[i].ping = ping;
        }
    }

    servers.sort_by(|a, b| match (a.ping, b.ping) {
        (Some(pa), Some(pb)) => pa.cmp(&pb),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    });

    let mut state_servers = state.servers.lock().await;
    *state_servers = servers.clone();

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
