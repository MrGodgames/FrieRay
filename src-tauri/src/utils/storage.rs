use serde::{de::DeserializeOwned, Serialize};
use std::path::PathBuf;

use crate::models::server::{Server, Subscription};
use crate::models::settings::AppSettings;

/// Get the config directory for FrieRay
fn config_dir() -> PathBuf {
    let dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("frieray");
    std::fs::create_dir_all(&dir).ok();
    dir
}

fn load_json<T: DeserializeOwned>(filename: &str) -> Option<T> {
    let path = config_dir().join(filename);
    let content = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&content).ok()
}

fn save_json<T: Serialize>(filename: &str, data: &T) -> Result<(), String> {
    let path = config_dir().join(filename);
    let json = serde_json::to_string_pretty(data).map_err(|e| format!("Serialize error: {}", e))?;
    std::fs::write(&path, json).map_err(|e| format!("Write error: {}", e))?;
    Ok(())
}

// ─── Public API ───

pub fn load_subscriptions() -> Vec<Subscription> {
    load_json("subscriptions.json").unwrap_or_default()
}

pub fn save_subscriptions(subs: &[Subscription]) -> Result<(), String> {
    save_json("subscriptions.json", &subs.to_vec())
}

pub fn load_servers() -> Vec<Server> {
    load_json("servers.json").unwrap_or_default()
}

pub fn save_servers(servers: &[Server]) -> Result<(), String> {
    save_json("servers.json", &servers.to_vec())
}

pub fn load_app_settings() -> AppSettings {
    load_json("settings.json").unwrap_or_default()
}

pub fn save_app_settings(settings: &AppSettings) -> Result<(), String> {
    save_json("settings.json", settings)
}

pub fn load_active_server_id() -> Option<String> {
    load_json::<String>("active_server.json")
}

pub fn save_active_server_id(id: &str) -> Result<(), String> {
    save_json("active_server.json", &id.to_string())
}
