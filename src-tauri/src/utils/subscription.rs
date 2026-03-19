use crate::models::server::{Protocol, Server, Subscription};
use crate::utils::vless;
use base64::{
    engine::general_purpose::{STANDARD, STANDARD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD},
    Engine,
};
use serde_json::Value;
use uuid::Uuid;

/// Fetch a subscription URL and return parsed servers
pub async fn fetch_subscription(sub: &Subscription) -> Result<Vec<Server>, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .user_agent("v2rayN/6.0")
        .danger_accept_invalid_certs(true)
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))?;

    log::info!("Fetching subscription: {} ({})", sub.name, sub.url);

    let response = client
        .get(&sub.url)
        .send()
        .await
        .map_err(|e| format!("Запрос не удался: {}", e))?;

    let status = response.status();
    if !status.is_success() {
        return Err(format!("HTTP ошибка: {}", status));
    }

    // Read as bytes first to handle encoding issues (gzip, non-utf8)
    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Ошибка чтения ответа: {}", e))?;

    log::info!("Subscription response: {} bytes", bytes.len());

    // Convert to string (lossy for non-utf8)
    let body = String::from_utf8_lossy(&bytes).to_string();

    let servers = parse_subscription_content(&body, &sub.id)?;
    log::info!("Parsed {} servers from subscription", servers.len());

    Ok(servers)
}

/// Parse subscription content (base64 encoded or plain text)
pub fn parse_subscription_content(
    content: &str,
    subscription_id: &str,
) -> Result<Vec<Server>, String> {
    let content = content.trim();

    if content.is_empty() {
        return Err("Пустой ответ от подписки".into());
    }

    // Determine if content is base64 or plain text
    let decoded_content = try_base64_decode(content).unwrap_or_else(|| content.to_string());

    let preview = &decoded_content[..decoded_content.len().min(300)];
    log::info!("Decoded content preview: {}", preview);

    let mut servers = Vec::new();

    for line in decoded_content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let result = if line.starts_with("vless://") {
            vless::parse_vless_url(line).ok()
        } else if line.starts_with("vmess://") {
            parse_vmess_url(line).ok()
        } else if line.starts_with("trojan://") {
            parse_trojan_url(line).ok()
        } else if line.starts_with("ss://") {
            parse_ss_url(line).ok()
        } else {
            log::warn!("Unknown line format: {}", &line[..line.len().min(50)]);
            None
        };

        if let Some(mut server) = result {
            server.subscription_id = Some(subscription_id.to_string());
            servers.push(server);
        }
    }

    Ok(servers)
}

/// Try multiple base64 engines to decode content
fn try_base64_decode(content: &str) -> Option<String> {
    let cleaned = content.replace(['\r', '\n', ' '], "");

    // Try each engine
    let try_decode = |data: &str| -> Option<String> {
        if let Ok(bytes) = STANDARD.decode(data) {
            if let Ok(text) = String::from_utf8(bytes) {
                if text.contains("://") {
                    return Some(text);
                }
            }
        }
        if let Ok(bytes) = STANDARD_NO_PAD.decode(data) {
            if let Ok(text) = String::from_utf8(bytes) {
                if text.contains("://") {
                    return Some(text);
                }
            }
        }
        if let Ok(bytes) = URL_SAFE.decode(data) {
            if let Ok(text) = String::from_utf8(bytes) {
                if text.contains("://") {
                    return Some(text);
                }
            }
        }
        if let Ok(bytes) = URL_SAFE_NO_PAD.decode(data) {
            if let Ok(text) = String::from_utf8(bytes) {
                if text.contains("://") {
                    return Some(text);
                }
            }
        }
        None
    };

    // Try as-is
    if let Some(result) = try_decode(&cleaned) {
        log::info!("Base64 decoded successfully");
        return Some(result);
    }

    // Try with padding
    for padding in &["=", "==", "==="] {
        let padded = format!("{}{}", cleaned, padding);
        if let Some(result) = try_decode(&padded) {
            log::info!("Base64 decoded with padding: {}", padding);
            return Some(result);
        }
    }

    log::info!("Content is not base64, treating as plain text");
    None
}

/// Parse a vmess:// URL (base64 JSON format)
fn parse_vmess_url(link: &str) -> Result<Server, String> {
    let data = link.strip_prefix("vmess://").ok_or("Not vmess")?;

    let decoded = try_base64_decode_single(data).ok_or("Failed to decode vmess base64")?;

    let json: Value =
        serde_json::from_str(&decoded).map_err(|e| format!("vmess JSON parse error: {}", e))?;

    let server = Server {
        id: Uuid::new_v4().to_string(),
        name: json["ps"].as_str().unwrap_or("vmess server").to_string(),
        address: json["add"].as_str().unwrap_or("").to_string(),
        port: json["port"]
            .as_str()
            .and_then(|p| p.parse().ok())
            .or_else(|| json["port"].as_u64().map(|p| p as u16))
            .unwrap_or(443),
        protocol: Protocol::Vmess,
        uuid: json["id"].as_str().unwrap_or("").to_string(),
        encryption: json["scy"].as_str().unwrap_or("auto").to_string(),
        flow: None,
        network: json["net"].as_str().unwrap_or("tcp").to_string(),
        security: json["tls"].as_str().unwrap_or("none").to_string(),
        sni: json["sni"].as_str().map(|s| s.to_string()),
        fingerprint: json["fp"].as_str().map(|s| s.to_string()),
        public_key: None,
        short_id: None,
        path: json["path"].as_str().map(|s| s.to_string()),
        host: json["host"].as_str().map(|s| s.to_string()),
        service_name: None,
        country: None,
        ping: None,
        speed_mbps: None,
        reachable: None,
        subscription_id: None,
    };

    Ok(server)
}

/// Parse a trojan:// URL
fn parse_trojan_url(link: &str) -> Result<Server, String> {
    let link = link.trim();
    let rest = link.strip_prefix("trojan://").ok_or("Not trojan")?;

    let (url_part, fragment) = match rest.split_once('#') {
        Some((u, f)) => (
            u,
            Some(urlencoding::decode(f).unwrap_or_default().to_string()),
        ),
        None => (rest, None),
    };

    let (user_host, query) = match url_part.split_once('?') {
        Some((uh, q)) => (uh, Some(q)),
        None => (url_part, None),
    };

    let (password, host_port) = user_host.split_once('@').ok_or("Missing @ in trojan")?;
    let (host, port_str) = host_port.rsplit_once(':').ok_or("Missing port")?;
    let port: u16 = port_str.parse().map_err(|_| "Invalid port")?;

    let params: std::collections::HashMap<String, String> = query
        .map(|q| {
            q.split('&')
                .filter_map(|p| p.split_once('='))
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect()
        })
        .unwrap_or_default();

    Ok(Server {
        id: Uuid::new_v4().to_string(),
        name: fragment.unwrap_or_else(|| format!("{}:{}", host, port)),
        address: host.to_string(),
        port,
        protocol: Protocol::Trojan,
        uuid: password.to_string(),
        encryption: "none".into(),
        flow: None,
        network: params.get("type").cloned().unwrap_or_else(|| "tcp".into()),
        security: params
            .get("security")
            .cloned()
            .unwrap_or_else(|| "tls".into()),
        sni: params.get("sni").cloned(),
        fingerprint: params.get("fp").cloned(),
        public_key: None,
        short_id: None,
        path: params.get("path").cloned(),
        host: params.get("host").cloned(),
        service_name: None,
        country: None,
        ping: None,
        speed_mbps: None,
        reachable: None,
        subscription_id: None,
    })
}

/// Parse a ss:// URL (Shadowsocks)
fn parse_ss_url(link: &str) -> Result<Server, String> {
    let rest = link.strip_prefix("ss://").ok_or("Not ss")?;

    let (encoded_part, fragment) = match rest.split_once('#') {
        Some((e, f)) => (
            e,
            Some(urlencoding::decode(f).unwrap_or_default().to_string()),
        ),
        None => (rest, None),
    };

    let (decoded_part, host_port) = if encoded_part.contains('@') {
        let (b64, hp) = encoded_part.split_once('@').unwrap();
        let decoded = try_base64_decode_single(b64).unwrap_or_else(|| b64.to_string());
        (decoded, Some(hp.to_string()))
    } else {
        let decoded =
            try_base64_decode_single(encoded_part).unwrap_or_else(|| encoded_part.to_string());
        (decoded, None)
    };

    let (method_pass, host, port) = if let Some(hp) = host_port {
        let hp_clean = hp.split('?').next().unwrap_or(&hp);
        let (host, port_str) = hp_clean.rsplit_once(':').ok_or("Missing ss port")?;
        let port: u16 = port_str.parse().map_err(|_| "Invalid ss port")?;
        (decoded_part, host.to_string(), port)
    } else {
        let (mp, hp) = decoded_part.rsplit_once('@').ok_or("Missing @ in ss")?;
        let (host, port_str) = hp.rsplit_once(':').ok_or("Missing ss port")?;
        let port: u16 = port_str.parse().map_err(|_| "Invalid ss port")?;
        (mp.to_string(), host.to_string(), port)
    };

    let (method, password) = method_pass
        .split_once(':')
        .unwrap_or(("aes-256-gcm", &method_pass));

    Ok(Server {
        id: Uuid::new_v4().to_string(),
        name: fragment.unwrap_or_else(|| format!("{}:{}", host, port)),
        address: host,
        port,
        protocol: Protocol::Shadowsocks,
        uuid: password.to_string(),
        encryption: method.to_string(),
        flow: None,
        network: "tcp".into(),
        security: "none".into(),
        sni: None,
        fingerprint: None,
        public_key: None,
        short_id: None,
        path: None,
        host: None,
        service_name: None,
        country: None,
        ping: None,
        speed_mbps: None,
        reachable: None,
        subscription_id: None,
    })
}

/// Decode a single base64 string (try all engine formats)
fn try_base64_decode_single(data: &str) -> Option<String> {
    let data = data.trim();

    // Try each engine directly (no dyn dispatch)
    for padding_suffix in &["", "=", "=="] {
        let input = if padding_suffix.is_empty() {
            data.to_string()
        } else {
            format!("{}{}", data, padding_suffix)
        };

        if let Ok(bytes) = STANDARD.decode(&input) {
            if let Ok(text) = String::from_utf8(bytes) {
                return Some(text);
            }
        }
        if let Ok(bytes) = STANDARD_NO_PAD.decode(&input) {
            if let Ok(text) = String::from_utf8(bytes) {
                return Some(text);
            }
        }
        if let Ok(bytes) = URL_SAFE.decode(&input) {
            if let Ok(text) = String::from_utf8(bytes) {
                return Some(text);
            }
        }
        if let Ok(bytes) = URL_SAFE_NO_PAD.decode(&input) {
            if let Ok(text) = String::from_utf8(bytes) {
                return Some(text);
            }
        }
    }

    None
}
