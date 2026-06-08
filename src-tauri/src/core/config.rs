use crate::models::server::{Protocol, Server};
use crate::models::settings::AppSettings;
use crate::models::xray_config::*;
use serde_json::json;

/// Generate a complete Xray-core config from server + settings
pub fn generate_xray_config(server: &Server, settings: &AppSettings, api_port: u16) -> XrayConfig {
    let proxy_outbound = build_proxy_outbound(server);
    let direct_outbound = build_direct_outbound();
    let block_outbound = build_block_outbound();

    let mut rules = Vec::new();

    // Rule: Zapret bypass domains → direct (if enabled)
    if settings.zapret.enabled && settings.zapret.bypass_vpn {
        let zapret_domains: Vec<String> = settings
            .zapret
            .services
            .iter()
            .filter(|s| s.enabled)
            .flat_map(|s| s.domains.iter().map(|d| format!("domain:{}", d)))
            .collect();

        if !zapret_domains.is_empty() {
            rules.push(RoutingRule {
                rule_type: "field".into(),
                domain: Some(zapret_domains),
                ip: None,
                port: None,
                outbound_tag: "direct".into(),
                inbound_tag: None,
            });
        }
    }

    // Rule: bypass private/LAN IPs (explicit CIDRs — no geoip.dat needed)
    rules.push(RoutingRule {
        rule_type: "field".into(),
        domain: None,
        ip: Some(vec![
            "10.0.0.0/8".into(),
            "172.16.0.0/12".into(),
            "192.168.0.0/16".into(),
            "127.0.0.0/8".into(),
            "100.64.0.0/10".into(),
            "169.254.0.0/16".into(),
            "fc00::/7".into(),
            "fe80::/10".into(),
            "::1/128".into(),
        ]),
        port: None,
        outbound_tag: "direct".into(),
        inbound_tag: None,
    });

    // Rule: block ads (optional, could be toggled)
    // rules.push(RoutingRule { ... geosite:category-ads-all → block });

    // Default rule: everything else → proxy
    // (Xray uses last outbound as default if no rule matches,
    //  but we put proxy first in outbounds list)

    // API routing rule (must be first)
    rules.insert(
        0,
        RoutingRule {
            rule_type: "field".into(),
            domain: None,
            ip: None,
            port: None,
            outbound_tag: "api".into(),
            inbound_tag: Some(vec!["api-in".into()]),
        },
    );

    // API outbound
    let api_outbound = Outbound {
        tag: "api".into(),
        protocol: "freedom".into(),
        settings: None,
        stream_settings: None,
    };

    XrayConfig {
        log: LogConfig {
            loglevel: "warning".into(),
        },
        stats: Some(json!({})),
        api: Some(json!({
            "tag": "api",
            "services": ["StatsService"]
        })),
        policy: Some(json!({
            "system": {
                "statsOutboundUplink": true,
                "statsOutboundDownlink": true
            }
        })),
        inbounds: vec![
            // SOCKS5 inbound
            Inbound {
                tag: "socks-in".into(),
                port: settings.proxy.socks_port,
                listen: "127.0.0.1".into(),
                protocol: "socks".into(),
                settings: Some(json!({
                    "udp": true,
                    "auth": "noauth"
                })),
                sniffing: Some(SniffingConfig {
                    enabled: true,
                    dest_override: vec!["http".into(), "tls".into(), "quic".into()],
                }),
            },
            // HTTP inbound
            Inbound {
                tag: "http-in".into(),
                port: settings.proxy.http_port,
                listen: "127.0.0.1".into(),
                protocol: "http".into(),
                settings: Some(json!({
                    "allowTransparent": false
                })),
                sniffing: None,
            },
            // API inbound (for stats)
            Inbound {
                tag: "api-in".into(),
                port: api_port,
                listen: "127.0.0.1".into(),
                protocol: "dokodemo-door".into(),
                settings: Some(json!({
                    "address": "127.0.0.1"
                })),
                sniffing: None,
            },
        ],
        outbounds: vec![
            proxy_outbound,
            direct_outbound,
            block_outbound,
            api_outbound,
        ],
        routing: RoutingConfig {
            domain_strategy: "IPIfNonMatch".into(),
            rules,
        },
        dns: Some(DnsConfig {
            servers: vec![json!(settings.dns.doh_server), json!("localhost")],
        }),
    }
}

fn build_proxy_outbound(server: &Server) -> Outbound {
    match server.protocol {
        Protocol::Vless => build_vless_outbound(server),
        Protocol::Vmess => build_vmess_outbound(server),
        Protocol::Trojan => build_trojan_outbound(server),
        Protocol::Shadowsocks => build_shadowsocks_outbound(server),
    }
}

fn build_vless_outbound(server: &Server) -> Outbound {
    let vnext = json!({
        "address": server.address,
        "port": server.port,
        "users": [{
            "id": server.uuid,
            "encryption": server.encryption,
            "flow": server.flow.clone().unwrap_or_default()
        }]
    });

    Outbound {
        tag: "proxy".into(),
        protocol: "vless".into(),
        settings: Some(json!({
            "vnext": [vnext]
        })),
        stream_settings: Some(build_stream_settings(server)),
    }
}

fn build_vmess_outbound(server: &Server) -> Outbound {
    let vnext = json!({
        "address": server.address,
        "port": server.port,
        "users": [{
            "id": server.uuid,
            "alterId": server.alter_id.unwrap_or(0),
            "security": normalize_vmess_security(&server.encryption)
        }]
    });

    Outbound {
        tag: "proxy".into(),
        protocol: "vmess".into(),
        settings: Some(json!({
            "vnext": [vnext]
        })),
        stream_settings: Some(build_stream_settings(server)),
    }
}

fn build_trojan_outbound(server: &Server) -> Outbound {
    let mut trojan_server = json!({
        "address": server.address,
        "port": server.port,
        "password": server.uuid,
    });

    if let Some(flow) = server.flow.as_deref().filter(|value| !value.is_empty()) {
        trojan_server["flow"] = json!(flow);
    }

    Outbound {
        tag: "proxy".into(),
        protocol: "trojan".into(),
        settings: Some(json!({
            "servers": [trojan_server]
        })),
        stream_settings: Some(build_stream_settings(server)),
    }
}

fn build_shadowsocks_outbound(server: &Server) -> Outbound {
    Outbound {
        tag: "proxy".into(),
        protocol: "shadowsocks".into(),
        settings: Some(json!({
            "servers": [{
                "address": server.address,
                "port": server.port,
                "method": server.encryption,
                "password": server.uuid
            }]
        })),
        stream_settings: None,
    }
}

fn build_stream_settings(server: &Server) -> StreamSettings {
    let network = normalize_network(&server.network);
    let security = normalize_security(&server.security);

    let tls_settings = if security == "tls" {
        Some(TlsSettings {
            server_name: server.sni.clone().unwrap_or_else(|| server.address.clone()),
            fingerprint: server.fingerprint.clone(),
            allow_insecure: false,
        })
    } else {
        None
    };

    let reality_settings = if security == "reality" {
        Some(RealitySettings {
            server_name: server.sni.clone().unwrap_or_default(),
            fingerprint: server
                .fingerprint
                .clone()
                .unwrap_or_else(|| "chrome".into()),
            public_key: server.public_key.clone().unwrap_or_default(),
            short_id: server.short_id.clone().unwrap_or_default(),
            spider_x: None,
        })
    } else {
        None
    };

    let ws_settings = if network == "ws" {
        Some(WsSettings {
            path: server.path.clone().unwrap_or_else(|| "/".into()),
            headers: server.host.as_ref().map(|h| json!({ "Host": h })),
        })
    } else {
        None
    };

    let grpc_settings = if network == "grpc" {
        Some(GrpcSettings {
            service_name: server.service_name.clone().unwrap_or_default(),
            multi_mode: false,
        })
    } else {
        None
    };

    let xhttp_settings = if network == "xhttp" {
        Some(XhttpSettings {
            path: server.path.clone().unwrap_or_else(|| "/".into()),
            host: server.host.clone(),
            mode: "auto".into(),
        })
    } else {
        None
    };

    StreamSettings {
        network,
        security,
        tls_settings,
        reality_settings,
        ws_settings,
        grpc_settings,
        xhttp_settings,
        tcp_settings: None,
    }
}

fn normalize_security(security: &str) -> String {
    let security = security.trim();
    if security.is_empty() {
        "none".into()
    } else {
        security.into()
    }
}

fn normalize_network(network: &str) -> String {
    match network.trim() {
        "splithttp" => "xhttp".into(),
        "" => "tcp".into(),
        value => value.into(),
    }
}

fn normalize_vmess_security(security: &str) -> String {
    match security.trim() {
        "" | "none" => "auto".into(),
        value => value.into(),
    }
}

fn build_direct_outbound() -> Outbound {
    Outbound {
        tag: "direct".into(),
        protocol: "freedom".into(),
        settings: None,
        stream_settings: None,
    }
}

fn build_block_outbound() -> Outbound {
    Outbound {
        tag: "block".into(),
        protocol: "blackhole".into(),
        settings: Some(json!({
            "response": { "type": "http" }
        })),
        stream_settings: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_server(protocol: Protocol) -> Server {
        Server {
            id: "server-id".into(),
            name: "test".into(),
            address: "example.com".into(),
            port: 443,
            protocol,
            uuid: "secret-or-id".into(),
            encryption: "auto".into(),
            alter_id: Some(0),
            flow: None,
            network: "ws".into(),
            security: "tls".into(),
            sni: Some("sni.example.com".into()),
            fingerprint: Some("chrome".into()),
            public_key: None,
            short_id: None,
            path: Some("/ws".into()),
            host: Some("host.example.com".into()),
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

    #[test]
    fn vmess_outbound_uses_vmess_protocol_and_alter_id() {
        let outbound = build_proxy_outbound(&test_server(Protocol::Vmess));
        let settings = outbound.settings.unwrap();

        assert_eq!(outbound.protocol, "vmess");
        assert_eq!(settings["vnext"][0]["users"][0]["alterId"], 0);
        assert_eq!(settings["vnext"][0]["users"][0]["security"], "auto");
        assert_eq!(outbound.stream_settings.unwrap().network, "ws");
    }

    #[test]
    fn trojan_outbound_uses_password_settings() {
        let outbound = build_proxy_outbound(&test_server(Protocol::Trojan));
        let settings = outbound.settings.unwrap();

        assert_eq!(outbound.protocol, "trojan");
        assert_eq!(settings["servers"][0]["password"], "secret-or-id");
        assert_eq!(outbound.stream_settings.unwrap().security, "tls");
    }

    #[test]
    fn shadowsocks_outbound_uses_method_and_password() {
        let mut server = test_server(Protocol::Shadowsocks);
        server.encryption = "aes-256-gcm".into();

        let outbound = build_proxy_outbound(&server);
        let settings = outbound.settings.unwrap();

        assert_eq!(outbound.protocol, "shadowsocks");
        assert_eq!(settings["servers"][0]["method"], "aes-256-gcm");
        assert_eq!(settings["servers"][0]["password"], "secret-or-id");
        assert!(outbound.stream_settings.is_none());
    }
}
