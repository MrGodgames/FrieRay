use tauri::State;
use crate::AppState;
use crate::models::server::Server;

#[tauri::command]
pub async fn connect(server: Server, state: State<'_, AppState>) -> Result<String, String> {
    state.logs.add("info", &format!("Подключение к {} ({}:{})...", server.name, server.address, server.port)).await;
    state.logs.add("info", &format!("Протокол: {:?}, Сеть: {}, Безопасность: {}", server.protocol, server.network, server.security)).await;

    let settings = state.settings.lock().await.clone();
    state.logs.add("info", &format!("SOCKS: {}, HTTP: {}, TUN: {}", settings.proxy.socks_port, settings.proxy.http_port,
        if settings.proxy.tun_mode { "ВКЛ" } else { "ВЫКЛ" })).await;

    // Start xray-core
    match state.xray.start(&server, &settings).await {
        Ok(()) => state.logs.add("success", "Xray-core запущен").await,
        Err(e) => {
            state.logs.add("error", &format!("Xray ошибка: {}", e)).await;
            return Err(format!("Не удалось подключиться: {}", e));
        }
    }

    // Start TUN or system proxy
    if settings.proxy.tun_mode {
        state.logs.add("info", "Запуск TUN режима...").await;
        match state.tun.start(settings.proxy.socks_port, &server.address).await {
            Ok(()) => state.logs.add("success", "TUN режим активирован — весь трафик идёт через VPN").await,
            Err(e) => {
                state.logs.add("error", &format!("TUN ошибка: {}", e)).await;
                // Fall back to system proxy
                state.logs.add("warn", "Откат на системный прокси...").await;
                if let Err(pe) = crate::core::proxy::set_system_proxy(settings.proxy.http_port, settings.proxy.socks_port) {
                    state.logs.add("warn", &format!("Прокси: {}", pe)).await;
                }
            }
        }
    } else if settings.proxy.system_proxy {
        match crate::core::proxy::set_system_proxy(settings.proxy.http_port, settings.proxy.socks_port) {
            Ok(()) => state.logs.add("success", "Системный прокси настроен").await,
            Err(e) => state.logs.add("warn", &format!("Прокси: {}", e)).await,
        }
    }

    let mut current = state.current_server.lock().await;
    *current = Some(server.clone());

    state.logs.add("success", &format!("Подключено к {}", server.name)).await;
    Ok(format!("Подключено к {}", server.name))
}

#[tauri::command]
pub async fn disconnect(state: State<'_, AppState>) -> Result<String, String> {
    state.logs.add("info", "Отключение...").await;

    // Stop TUN first (restores routes)
    if let Err(e) = state.tun.stop().await {
        state.logs.add("warn", &format!("TUN stop: {}", e)).await;
    }

    // Stop xray
    state.xray.stop().await?;
    state.logs.add("info", "Xray-core остановлен").await;

    // Always unset system proxy on disconnect, just to be safe
    // in case it was set by a fallback or previous session
    if let Err(e) = crate::core::proxy::unset_system_proxy() {
        state.logs.add("warn", &format!("Прокси сброс: {}", e)).await;
    } else {
        state.logs.add("info", "Системный прокси сброшен").await;
    }

    let mut current = state.current_server.lock().await;
    *current = None;

    state.logs.add("info", "Отключено").await;
    Ok("Отключено".into())
}

#[tauri::command]
pub async fn get_connection_status(state: State<'_, AppState>) -> Result<bool, String> {
    Ok(state.xray.is_running().await)
}

#[tauri::command]
pub async fn get_current_server(state: State<'_, AppState>) -> Result<Option<Server>, String> {
    let server = state.current_server.lock().await;
    Ok(server.clone())
}
