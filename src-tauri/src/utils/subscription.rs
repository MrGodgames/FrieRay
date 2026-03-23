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

    if looks_like_json_config(&decoded_content) {
        return parse_json_subscription_config(&decoded_content, subscription_id);
    }

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
                if is_supported_subscription_payload(&text) {
                    return Some(text);
                }
            }
        }
        if let Ok(bytes) = STANDARD_NO_PAD.decode(data) {
            if let Ok(text) = String::from_utf8(bytes) {
                if is_supported_subscription_payload(&text) {
                    return Some(text);
                }
            }
        }
        if let Ok(bytes) = URL_SAFE.decode(data) {
            if let Ok(text) = String::from_utf8(bytes) {
                if is_supported_subscription_payload(&text) {
                    return Some(text);
                }
            }
        }
        if let Ok(bytes) = URL_SAFE_NO_PAD.decode(data) {
            if let Ok(text) = String::from_utf8(bytes) {
                if is_supported_subscription_payload(&text) {
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

fn is_supported_subscription_payload(text: &str) -> bool {
    let trimmed = text.trim();
    trimmed.contains("://") || looks_like_json_config(trimmed)
}

fn looks_like_json_config(content: &str) -> bool {
    let trimmed = content.trim_start();
    trimmed.starts_with('{') || trimmed.starts_with('[')
}

fn parse_json_subscription_config(
    content: &str,
    subscription_id: &str,
) -> Result<Vec<Server>, String> {
    let root: Value =
        serde_json::from_str(content).map_err(|e| format!("JSON parse error: {}", e))?;

    let mut embedded_links = Vec::new();
    collect_embedded_links(&root, &mut embedded_links);
    if !embedded_links.is_empty() {
        let mut servers = Vec::new();
        for link in embedded_links {
            let result = if link.starts_with("vless://") {
                vless::parse_vless_url(&link).ok()
            } else if link.starts_with("vmess://") {
                parse_vmess_url(&link).ok()
            } else if link.starts_with("trojan://") {
                parse_trojan_url(&link).ok()
            } else if link.starts_with("ss://") {
                parse_ss_url(&link).ok()
            } else {
                None
            };

            if let Some(mut server) = result {
                server.subscription_id = Some(subscription_id.to_string());
                servers.push(server);
            }
        }

        if !servers.is_empty() {
            return Ok(servers);
        }
    }

    let root_remarks = root
        .get("remarks")
        .and_then(Value::as_str)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    let outbounds = root
        .get("outbounds")
        .and_then(Value::as_array)
        .or_else(|| root.get("proxies").and_then(Value::as_array))
        .ok_or("JSON-конфиг не содержит outbounds/proxies")?;

    let mut servers = Vec::new();
    for outbound in outbounds {
        if let Some(mut server) = parse_json_outbound(outbound, root_remarks.as_deref())? {
            server.subscription_id = Some(subscription_id.to_string());
            servers.push(server);
        }
    }

    Ok(servers)
}

fn parse_json_outbound(
    outbound: &Value,
    root_remarks: Option<&str>,
) -> Result<Option<Server>, String> {
    let protocol_str = outbound
        .get("protocol")
        .or_else(|| outbound.get("type"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_lowercase();

    let Some(protocol) = (match protocol_str.as_str() {
        "vless" => Some(Protocol::Vless),
        "vmess" => Some(Protocol::Vmess),
        "trojan" => Some(Protocol::Trojan),
        "shadowsocks" => Some(Protocol::Shadowsocks),
        _ => None,
    }) else {
        return Ok(None);
    };

    let stream = outbound.get("streamSettings").unwrap_or(&Value::Null);
    let transport = outbound.get("transport").unwrap_or(&Value::Null);
    let tls = outbound.get("tls").unwrap_or(&Value::Null);
    let network = stream
        .get("network")
        .and_then(Value::as_str)
        .or_else(|| transport.get("type").and_then(Value::as_str))
        .unwrap_or("tcp")
        .to_string();
    let security = stream
        .get("security")
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| detect_json_security(outbound));
    let security_block = if security == "reality" {
        stream.get("realitySettings").unwrap_or(&Value::Null)
    } else if security == "tls" {
        stream.get("tlsSettings").unwrap_or(&Value::Null)
    } else if tls.get("enabled").and_then(Value::as_bool) == Some(true) {
        tls
    } else {
        &Value::Null
    };

    let settings = outbound.get("settings").unwrap_or(&Value::Null);
    let tag = outbound.get("tag").and_then(Value::as_str);
    let outbound_remarks = outbound.get("remarks").and_then(Value::as_str);

    let mut server = match protocol {
        Protocol::Vless | Protocol::Vmess => {
            let vnext = settings
                .get("vnext")
                .and_then(Value::as_array)
                .and_then(|items| items.first());
            let user = vnext
                .and_then(|value| value.get("users"))
                .and_then(Value::as_array)
                .and_then(|items| items.first());

            Server {
                id: Uuid::new_v4().to_string(),
                name: build_json_server_name(root_remarks, outbound_remarks, tag, &protocol_str),
                address: value_as_string(vnext.and_then(|value| value.get("address")))
                    .or_else(|| value_as_string(outbound.get("server")))
                    .unwrap_or_default(),
                port: value_as_u16(vnext.and_then(|value| value.get("port")))
                    .or_else(|| value_as_u16(outbound.get("server_port")))
                    .unwrap_or(443),
                protocol,
                uuid: value_as_string(user.and_then(|value| value.get("id")))
                    .or_else(|| value_as_string(outbound.get("uuid")))
                    .unwrap_or_default(),
                encryption: value_as_string(user.and_then(|value| value.get("encryption")))
                    .or_else(|| value_as_string(outbound.get("security")))
                    .unwrap_or_else(|| {
                        if protocol_str == "vless" {
                            "none".into()
                        } else {
                            "auto".into()
                        }
                    }),
                flow: value_as_string(user.and_then(|value| value.get("flow")))
                    .or_else(|| value_as_string(outbound.get("flow"))),
                network,
                security,
                sni: value_as_string(security_block.get("serverName"))
                    .or_else(|| value_as_string(security_block.get("server_name")))
                    .or_else(|| value_as_string(tls.get("server_name"))),
                fingerprint: value_as_string(security_block.get("fingerprint"))
                    .or_else(|| value_as_string(security_block.get("client_fingerprint")))
                    .or_else(|| {
                        value_as_string(tls.get("utls").and_then(|value| value.get("fingerprint")))
                    }),
                public_key: value_as_string(
                    stream
                        .get("realitySettings")
                        .and_then(|value| value.get("publicKey")),
                )
                .or_else(|| {
                    value_as_string(tls.get("reality").and_then(|value| value.get("public_key")))
                }),
                short_id: value_as_string(
                    stream
                        .get("realitySettings")
                        .and_then(|value| value.get("shortId")),
                )
                .or_else(|| {
                    value_as_string(tls.get("reality").and_then(|value| value.get("short_id")))
                }),
                path: extract_path(stream).or_else(|| extract_transport_path(transport)),
                host: extract_host(stream).or_else(|| extract_transport_host(transport)),
                service_name: value_as_string(
                    stream
                        .get("grpcSettings")
                        .and_then(|value| value.get("serviceName")),
                )
                .or_else(|| value_as_string(transport.get("service_name"))),
                country: None,
                ping: None,
                speed_mbps: None,
                reachable: None,
                speed_reachable: None,
                ping_checking: false,
                speed_checking: false,
                subscription_id: None,
            }
        }
        Protocol::Trojan => {
            let server_entry = settings
                .get("servers")
                .and_then(Value::as_array)
                .and_then(|items| items.first());

            Server {
                id: Uuid::new_v4().to_string(),
                name: build_json_server_name(root_remarks, outbound_remarks, tag, &protocol_str),
                address: value_as_string(server_entry.and_then(|value| value.get("address")))
                    .or_else(|| value_as_string(outbound.get("server")))
                    .unwrap_or_default(),
                port: value_as_u16(server_entry.and_then(|value| value.get("port")))
                    .or_else(|| value_as_u16(outbound.get("server_port")))
                    .unwrap_or(443),
                protocol,
                uuid: value_as_string(server_entry.and_then(|value| value.get("password")))
                    .or_else(|| value_as_string(outbound.get("password")))
                    .unwrap_or_default(),
                encryption: "none".into(),
                flow: value_as_string(server_entry.and_then(|value| value.get("flow")))
                    .or_else(|| value_as_string(outbound.get("flow"))),
                network,
                security,
                sni: value_as_string(security_block.get("serverName"))
                    .or_else(|| value_as_string(security_block.get("server_name")))
                    .or_else(|| value_as_string(tls.get("server_name"))),
                fingerprint: value_as_string(security_block.get("fingerprint"))
                    .or_else(|| value_as_string(security_block.get("client_fingerprint")))
                    .or_else(|| {
                        value_as_string(tls.get("utls").and_then(|value| value.get("fingerprint")))
                    }),
                public_key: None,
                short_id: None,
                path: extract_path(stream).or_else(|| extract_transport_path(transport)),
                host: extract_host(stream).or_else(|| extract_transport_host(transport)),
                service_name: value_as_string(
                    stream
                        .get("grpcSettings")
                        .and_then(|value| value.get("serviceName")),
                )
                .or_else(|| value_as_string(transport.get("service_name"))),
                country: None,
                ping: None,
                speed_mbps: None,
                reachable: None,
                speed_reachable: None,
                ping_checking: false,
                speed_checking: false,
                subscription_id: None,
            }
        }
        Protocol::Shadowsocks => {
            let server_entry = settings
                .get("servers")
                .and_then(Value::as_array)
                .and_then(|items| items.first());

            Server {
                id: Uuid::new_v4().to_string(),
                name: build_json_server_name(root_remarks, outbound_remarks, tag, &protocol_str),
                address: value_as_string(server_entry.and_then(|value| value.get("address")))
                    .or_else(|| value_as_string(outbound.get("server")))
                    .unwrap_or_default(),
                port: value_as_u16(server_entry.and_then(|value| value.get("port")))
                    .or_else(|| value_as_u16(outbound.get("server_port")))
                    .unwrap_or(8388),
                protocol,
                uuid: value_as_string(server_entry.and_then(|value| value.get("password")))
                    .or_else(|| value_as_string(outbound.get("password")))
                    .unwrap_or_default(),
                encryption: value_as_string(server_entry.and_then(|value| value.get("method")))
                    .or_else(|| value_as_string(outbound.get("method")))
                    .unwrap_or_else(|| "aes-256-gcm".into()),
                flow: None,
                network,
                security,
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
                speed_reachable: None,
                ping_checking: false,
                speed_checking: false,
                subscription_id: None,
            }
        }
    };

    if server.name.trim().is_empty() {
        server.name = format!("{} {}:{}", protocol_str, server.address, server.port);
    }

    if server.address.trim().is_empty() {
        return Ok(None);
    }

    Ok(Some(server))
}

fn build_json_server_name(
    root_remarks: Option<&str>,
    outbound_remarks: Option<&str>,
    tag: Option<&str>,
    protocol: &str,
) -> String {
    let outbound_label = outbound_remarks
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .or_else(|| tag.map(str::trim).filter(|value| !value.is_empty()));

    match (root_remarks, outbound_label) {
        (Some(root), Some(label)) if root != label => format!("{} [{}]", root, label),
        (Some(root), _) => root.to_string(),
        (_, Some(label)) => label.to_string(),
        _ => protocol.to_uppercase(),
    }
}

fn value_as_string(value: Option<&Value>) -> Option<String> {
    match value {
        Some(Value::String(value)) => Some(value.clone()),
        Some(Value::Number(value)) => Some(value.to_string()),
        Some(Value::Array(items)) => items.first().and_then(|item| value_as_string(Some(item))),
        _ => None,
    }
}

fn value_as_u16(value: Option<&Value>) -> Option<u16> {
    match value {
        Some(Value::Number(number)) => number.as_u64().and_then(|value| u16::try_from(value).ok()),
        Some(Value::String(text)) => text.parse().ok(),
        _ => None,
    }
}

fn extract_path(stream: &Value) -> Option<String> {
    value_as_string(stream.get("wsSettings").and_then(|value| value.get("path")))
        .or_else(|| {
            value_as_string(
                stream
                    .get("xhttpSettings")
                    .and_then(|value| value.get("path")),
            )
        })
        .or_else(|| {
            value_as_string(
                stream
                    .get("httpupgradeSettings")
                    .and_then(|value| value.get("path")),
            )
        })
}

fn extract_host(stream: &Value) -> Option<String> {
    value_as_string(
        stream
            .get("wsSettings")
            .and_then(|value| value.get("headers"))
            .and_then(|value| value.get("Host")),
    )
    .or_else(|| {
        value_as_string(
            stream
                .get("xhttpSettings")
                .and_then(|value| value.get("host")),
        )
    })
    .or_else(|| {
        value_as_string(
            stream
                .get("httpSettings")
                .and_then(|value| value.get("host")),
        )
    })
}

fn extract_transport_path(transport: &Value) -> Option<String> {
    value_as_string(transport.get("path"))
}

fn extract_transport_host(transport: &Value) -> Option<String> {
    value_as_string(transport.get("host"))
        .or_else(|| value_as_string(transport.get("headers").and_then(|value| value.get("Host"))))
}

fn detect_json_security(outbound: &Value) -> String {
    let tls = outbound.get("tls").unwrap_or(&Value::Null);
    if tls
        .get("reality")
        .and_then(|value| value.get("enabled"))
        .and_then(Value::as_bool)
        == Some(true)
        || tls
            .get("reality")
            .and_then(|value| value.get("public_key"))
            .is_some()
    {
        "reality".into()
    } else if tls.get("enabled").and_then(Value::as_bool) == Some(true) {
        "tls".into()
    } else {
        "none".into()
    }
}

fn collect_embedded_links(value: &Value, links: &mut Vec<String>) {
    match value {
        Value::String(text) => {
            let text = text.trim();
            if text.starts_with("vless://")
                || text.starts_with("vmess://")
                || text.starts_with("trojan://")
                || text.starts_with("ss://")
            {
                links.push(text.to_string());
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_embedded_links(item, links);
            }
        }
        Value::Object(map) => {
            for item in map.values() {
                collect_embedded_links(item, links);
            }
        }
        _ => {}
    }
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
        speed_reachable: None,
        ping_checking: false,
        speed_checking: false,
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
        speed_reachable: None,
        ping_checking: false,
        speed_checking: false,
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
        speed_reachable: None,
        ping_checking: false,
        speed_checking: false,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_xray_json_subscription() {
        let json = r#"{
          "remarks": "Test Config",
          "outbounds": [
            {
              "tag": "proxy-a",
              "protocol": "vless",
              "settings": {
                "vnext": [
                  {
                    "address": "example.com",
                    "port": 443,
                    "users": [
                      {
                        "id": "11111111-1111-1111-1111-111111111111",
                        "encryption": "none",
                        "flow": "xtls-rprx-vision"
                      }
                    ]
                  }
                ]
              },
              "streamSettings": {
                "network": "tcp",
                "security": "reality",
                "realitySettings": {
                  "serverName": "cdn.example.com",
                  "publicKey": "pubkey",
                  "shortId": "abcd",
                  "fingerprint": "chrome"
                }
              }
            },
            {
              "tag": "direct",
              "protocol": "freedom"
            }
          ]
        }"#;

        let servers = parse_subscription_content(json, "sub-1").unwrap();
        assert_eq!(servers.len(), 1);
        assert_eq!(servers[0].address, "example.com");
        assert_eq!(servers[0].security, "reality");
        assert_eq!(servers[0].public_key.as_deref(), Some("pubkey"));
        assert_eq!(servers[0].short_id.as_deref(), Some("abcd"));
        assert_eq!(servers[0].subscription_id.as_deref(), Some("sub-1"));
    }
}
