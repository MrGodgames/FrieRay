mod commands;
mod core;
mod models;
mod utils;

use core::tun::TunManager;
use core::xray::XrayManager;
use models::server::{Server, Subscription};
use models::settings::AppSettings;
use tokio::sync::Mutex;
use utils::log_buffer::LogBuffer;
use utils::storage;

/// Shared application state
pub struct AppState {
    pub xray: XrayManager,
    pub tun: TunManager,
    pub settings: Mutex<AppSettings>,
    pub servers: Mutex<Vec<Server>>,
    pub subscriptions: Mutex<Vec<Subscription>>,
    pub current_server: Mutex<Option<Server>>,
    pub active_server: Mutex<Option<Server>>,
    pub logs: LogBuffer,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_secs()
        .init();

    log::info!("FrieRay starting...");

    let saved_subs = storage::load_subscriptions();
    let saved_servers = storage::load_servers();
    let saved_settings = storage::load_app_settings();
    let active_server = storage::load_active_server_id()
        .and_then(|id| saved_servers.iter().find(|s| s.id == id).cloned());

    log::info!(
        "Loaded: {} subs, {} servers, active: {}",
        saved_subs.len(),
        saved_servers.len(),
        active_server
            .as_ref()
            .map(|s| s.name.as_str())
            .unwrap_or("none")
    );

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .manage(AppState {
            xray: XrayManager::new(),
            tun: TunManager::new(),
            settings: Mutex::new(saved_settings),
            servers: Mutex::new(saved_servers),
            subscriptions: Mutex::new(saved_subs),
            current_server: Mutex::new(None),
            active_server: Mutex::new(active_server),
            logs: LogBuffer::new(),
        })
        .invoke_handler(tauri::generate_handler![
            // Connection
            commands::connection::connect,
            commands::connection::disconnect,
            commands::connection::get_connection_status,
            commands::connection::get_current_server,
            // Servers
            commands::servers::add_subscription,
            commands::servers::remove_subscription,
            commands::servers::update_subscriptions,
            commands::servers::get_servers,
            commands::servers::get_subscriptions,
            commands::servers::parse_link,
            commands::servers::set_active_server,
            commands::servers::get_active_server,
            commands::servers::ping_server,
            commands::servers::ping_all_servers,
            // System
            commands::system::save_settings,
            commands::system::load_settings,
            commands::system::get_installed_apps,
            commands::system::install_tun_helper,
            commands::system::is_tun_ready,
            // Logs & Stats
            commands::logs::get_logs,
            commands::logs::clear_logs,
            commands::logs::get_traffic_stats,
            commands::logs::speed_test,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
