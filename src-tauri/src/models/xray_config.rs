use serde::{Deserialize, Serialize};

/// Полная структура конфигурации Xray-core
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XrayConfig {
    pub log: LogConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stats: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy: Option<serde_json::Value>,
    pub inbounds: Vec<Inbound>,
    pub outbounds: Vec<Outbound>,
    pub routing: RoutingConfig,
    pub dns: Option<DnsConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogConfig {
    pub loglevel: String,
}

// ─── Inbounds ───

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Inbound {
    pub tag: String,
    pub port: u16,
    pub listen: String,
    pub protocol: String,
    pub settings: Option<serde_json::Value>,
    pub sniffing: Option<SniffingConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SniffingConfig {
    pub enabled: bool,
    #[serde(rename = "destOverride")]
    pub dest_override: Vec<String>,
}

// ─── Outbounds ───

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Outbound {
    pub tag: String,
    pub protocol: String,
    pub settings: Option<serde_json::Value>,
    #[serde(rename = "streamSettings", skip_serializing_if = "Option::is_none")]
    pub stream_settings: Option<StreamSettings>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamSettings {
    pub network: String,
    pub security: String,
    #[serde(rename = "tlsSettings", skip_serializing_if = "Option::is_none")]
    pub tls_settings: Option<TlsSettings>,
    #[serde(rename = "realitySettings", skip_serializing_if = "Option::is_none")]
    pub reality_settings: Option<RealitySettings>,
    #[serde(rename = "wsSettings", skip_serializing_if = "Option::is_none")]
    pub ws_settings: Option<WsSettings>,
    #[serde(rename = "grpcSettings", skip_serializing_if = "Option::is_none")]
    pub grpc_settings: Option<GrpcSettings>,
    #[serde(rename = "tcpSettings", skip_serializing_if = "Option::is_none")]
    pub tcp_settings: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsSettings {
    #[serde(rename = "serverName")]
    pub server_name: String,
    pub fingerprint: Option<String>,
    #[serde(rename = "allowInsecure")]
    pub allow_insecure: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealitySettings {
    #[serde(rename = "serverName")]
    pub server_name: String,
    pub fingerprint: String,
    #[serde(rename = "publicKey")]
    pub public_key: String,
    #[serde(rename = "shortId")]
    pub short_id: String,
    #[serde(rename = "spiderX")]
    pub spider_x: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsSettings {
    pub path: String,
    pub headers: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrpcSettings {
    #[serde(rename = "serviceName")]
    pub service_name: String,
    #[serde(rename = "multiMode")]
    pub multi_mode: bool,
}

// ─── Routing ───

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingConfig {
    #[serde(rename = "domainStrategy")]
    pub domain_strategy: String,
    pub rules: Vec<RoutingRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingRule {
    #[serde(rename = "type")]
    pub rule_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<String>,
    #[serde(rename = "outboundTag")]
    pub outbound_tag: String,
    #[serde(rename = "inboundTag", skip_serializing_if = "Option::is_none")]
    pub inbound_tag: Option<Vec<String>>,
}

// ─── DNS ───

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsConfig {
    pub servers: Vec<serde_json::Value>,
}
