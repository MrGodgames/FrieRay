use crate::models::server::{Protocol, Server};
use url::Url;
use uuid::Uuid;

/// Parse a vless:// URL into a Server struct
///
/// Format: vless://UUID@host:port?params#name
///
/// Example:
/// vless://uuid@server.com:443?type=tcp&security=reality&sni=google.com&fp=chrome&pbk=KEY&sid=SID&flow=xtls-rprx-vision#ServerName
pub fn parse_vless_url(link: &str) -> Result<Server, String> {
    let link = link.trim();
    if !link.starts_with("vless://") {
        return Err("Not a VLESS link".into());
    }

    // Convert vless:// to https:// for URL parsing
    let https_url = link.replacen("vless://", "https://dummy:dummy@", 1);

    // Split off the fragment (server name) before parsing
    let (url_part, fragment) = match https_url.split_once('#') {
        Some((u, f)) => (
            u.to_string(),
            Some(urlencoding::decode(f).unwrap_or_default().to_string()),
        ),
        None => (https_url, None),
    };

    let parsed = Url::parse(&url_part).map_err(|e| format!("URL parse error: {}", e))?;

    // Extract UUID from the original link
    let uuid_str = link
        .strip_prefix("vless://")
        .and_then(|s| s.split('@').next())
        .ok_or("Missing UUID")?
        .to_string();

    let host = parsed.host_str().ok_or("Missing host")?.to_string();
    let port = parsed.port().unwrap_or(443);

    // Parse query parameters
    let params: std::collections::HashMap<String, String> = parsed
        .query_pairs()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();

    let server = Server {
        id: Uuid::new_v4().to_string(),
        name: fragment.unwrap_or_else(|| format!("{}:{}", host, port)),
        address: host,
        port,
        protocol: Protocol::Vless,
        uuid: uuid_str,
        encryption: params
            .get("encryption")
            .cloned()
            .unwrap_or_else(|| "none".into()),
        flow: params.get("flow").cloned(),
        network: params.get("type").cloned().unwrap_or_else(|| "tcp".into()),
        security: params
            .get("security")
            .cloned()
            .unwrap_or_else(|| "none".into()),
        sni: params.get("sni").cloned(),
        fingerprint: params.get("fp").cloned(),
        public_key: params.get("pbk").cloned(),
        short_id: params.get("sid").cloned(),
        path: params.get("path").cloned(),
        host: params.get("host").cloned(),
        service_name: params.get("serviceName").cloned(),
        country: None,
        ping: None,
        subscription_id: None,
    };

    Ok(server)
}

/// Basic link type detection
pub fn detect_protocol(link: &str) -> Option<Protocol> {
    let link = link.trim().to_lowercase();
    if link.starts_with("vless://") {
        Some(Protocol::Vless)
    } else if link.starts_with("vmess://") {
        Some(Protocol::Vmess)
    } else if link.starts_with("trojan://") {
        Some(Protocol::Trojan)
    } else if link.starts_with("ss://") {
        Some(Protocol::Shadowsocks)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_vless_basic() {
        let link = "vless://550e8400-e29b-41d4-a716-446655440000@server.com:443?type=tcp&security=tls&sni=google.com&fp=chrome#TestServer";
        let server = parse_vless_url(link).unwrap();
        assert_eq!(server.address, "server.com");
        assert_eq!(server.port, 443);
        assert_eq!(server.uuid, "550e8400-e29b-41d4-a716-446655440000");
        assert_eq!(server.network, "tcp");
        assert_eq!(server.security, "tls");
        assert_eq!(server.sni.as_deref(), Some("google.com"));
        assert_eq!(server.name, "TestServer");
    }

    #[test]
    fn test_parse_vless_reality() {
        let link = "vless://uuid@host.com:443?type=tcp&security=reality&sni=www.google.com&fp=chrome&pbk=publickey123&sid=shortid123&flow=xtls-rprx-vision#RealityServer";
        let server = parse_vless_url(link).unwrap();
        assert_eq!(server.security, "reality");
        assert_eq!(server.public_key.as_deref(), Some("publickey123"));
        assert_eq!(server.short_id.as_deref(), Some("shortid123"));
        assert_eq!(server.flow.as_deref(), Some("xtls-rprx-vision"));
    }
}
