use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub general: GeneralSettings,
    pub proxy: ProxySettings,
    pub dns: DnsSettings,
    pub zapret: ZapretSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralSettings {
    pub auto_connect: bool,
    pub start_minimized: bool,
    #[serde(default)]
    pub launch_at_login: bool,
    pub auto_update_subs: bool,
    pub auto_update_interval_hours: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxySettings {
    pub system_proxy: bool,
    pub tun_mode: bool,
    pub socks_port: u16,
    pub http_port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsSettings {
    pub doh_server: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZapretSettings {
    pub enabled: bool,
    pub strategy: ZapretStrategy,
    pub bypass_vpn: bool,
    pub services: Vec<ZapretService>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ZapretStrategy {
    Auto,
    Split,
    Fake,
    Desync,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZapretService {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub domains: Vec<String>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            general: GeneralSettings {
                auto_connect: false,
                start_minimized: false,
                launch_at_login: false,
                auto_update_subs: true,
                auto_update_interval_hours: 6,
            },
            proxy: ProxySettings {
                system_proxy: true,
                tun_mode: false,
                socks_port: 10808,
                http_port: 10809,
            },
            dns: DnsSettings {
                doh_server: "https://dns.google/dns-query".to_string(),
            },
            zapret: ZapretSettings {
                enabled: false,
                strategy: ZapretStrategy::Auto,
                bypass_vpn: true,
                services: default_zapret_services(),
            },
        }
    }
}

fn default_zapret_services() -> Vec<ZapretService> {
    vec![
        ZapretService {
            id: "youtube".to_string(),
            name: "YouTube".to_string(),
            enabled: true,
            domains: vec![
                "youtube.com".into(),
                "googlevideo.com".into(),
                "ytimg.com".into(),
                "ggpht.com".into(),
                "youtu.be".into(),
            ],
        },
        ZapretService {
            id: "discord".to_string(),
            name: "Discord".to_string(),
            enabled: true,
            domains: vec![
                "discord.com".into(),
                "discord.gg".into(),
                "discordapp.com".into(),
                "discord.media".into(),
                "discordapp.net".into(),
            ],
        },
        ZapretService {
            id: "telegram".to_string(),
            name: "Telegram".to_string(),
            enabled: true,
            domains: vec![
                "telegram.org".into(),
                "t.me".into(),
                "core.telegram.org".into(),
                "web.telegram.org".into(),
            ],
        },
    ]
}
