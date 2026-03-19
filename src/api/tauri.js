/**
 * FrieRay API — Frontend bridge to Tauri/Rust backend
 */

const isTauri = typeof window !== 'undefined' && window.__TAURI_INTERNALS__;

async function invoke(command, args = {}) {
    if (isTauri) {
        const { invoke } = await import('@tauri-apps/api/core');
        return invoke(command, args);
    }
    console.warn(`[FrieRay] Tauri not available, mock: ${command}`, args);
    return null;
}

// ─── Connection ───
export async function connect(server) { return invoke('connect', { server }); }
export async function disconnect() { return invoke('disconnect'); }
export async function getConnectionStatus() { return (await invoke('get_connection_status')) ?? false; }
export async function getCurrentServer() { return invoke('get_current_server'); }

// ─── Servers ───
export async function addSubscription(name, url) { return invoke('add_subscription', { name, url }); }
export async function removeSubscription(id) { return invoke('remove_subscription', { id }); }
export async function updateSubscriptions() { return invoke('update_subscriptions'); }
export async function getServers() { return (await invoke('get_servers')) ?? []; }
export async function getSubscriptions() { return (await invoke('get_subscriptions')) ?? []; }
export async function parseLink(link) { return invoke('parse_link', { link }); }
export async function setActiveServer(serverId) { return invoke('set_active_server', { serverId }); }
export async function getActiveServer() { return invoke('get_active_server'); }
export async function pingServer(address, port) { return invoke('ping_server', { address, port }); }
export async function pingAllServers() { return (await invoke('ping_all_servers')) ?? []; }

// ─── Settings ───
export async function saveSettings(settings) { return invoke('save_settings', { settings }); }
export async function loadSettings() { return invoke('load_settings'); }

// ─── System ───
export async function getInstalledApps() { return (await invoke('get_installed_apps')) ?? []; }
export async function installTunHelper() { return invoke('install_tun_helper'); }
export async function isTunReady() { return (await invoke('is_tun_ready')) ?? false; }

// ─── Logs & Stats ───
export async function getLogs() { return (await invoke('get_logs')) ?? []; }
export async function clearLogs() { return invoke('clear_logs'); }
export async function getTrafficStats() { return invoke('get_traffic_stats'); }
export async function speedTest() { return invoke('speed_test'); }

export { isTauri };
