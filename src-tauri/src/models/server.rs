use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Server {
    pub id: String,
    pub name: String,
    pub address: String,
    pub port: u16,
    pub protocol: Protocol,
    pub uuid: String,
    pub encryption: String,
    pub flow: Option<String>,
    pub network: String,  // tcp, ws, grpc, h2
    pub security: String, // tls, reality, none
    pub sni: Option<String>,
    pub fingerprint: Option<String>,
    pub public_key: Option<String>,   // reality
    pub short_id: Option<String>,     // reality
    pub path: Option<String>,         // ws path
    pub host: Option<String>,         // ws host
    pub service_name: Option<String>, // grpc
    pub country: Option<String>,
    pub ping: Option<u32>,
    pub subscription_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
    Vless,
    Vmess,
    Trojan,
    Shadowsocks,
}

impl std::fmt::Display for Protocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Protocol::Vless => write!(f, "vless"),
            Protocol::Vmess => write!(f, "vmess"),
            Protocol::Trojan => write!(f, "trojan"),
            Protocol::Shadowsocks => write!(f, "ss"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    pub id: String,
    pub name: String,
    pub url: String,
    pub last_update: Option<String>,
    pub server_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionStatus {
    pub connected: bool,
    pub connecting: bool,
    pub server_name: Option<String>,
    pub duration_secs: u64,
    pub download_bytes: u64,
    pub upload_bytes: u64,
}

impl Default for ConnectionStatus {
    fn default() -> Self {
        Self {
            connected: false,
            connecting: false,
            server_name: None,
            duration_secs: 0,
            download_bytes: 0,
            upload_bytes: 0,
        }
    }
}
