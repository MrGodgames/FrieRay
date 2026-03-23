use crate::models::server::Server;
use crate::AppState;
use tauri::{Emitter, Manager, State};

pub async fn connect_best_server_with_app(app: &tauri::AppHandle) -> Result<String, String> {
    let state = app.state::<AppState>();
    let selection = crate::commands::servers::choose_best_server(app, &state).await?;

    {
        let mut active = state.active_server.lock().await;
        *active = Some(selection.server.clone());
    }
    crate::utils::storage::save_active_server_id(&selection.server.id)?;

    state
        .logs
        .add(
            "info",
            &format!(
                "Автовыбор сервера: {} {}",
                selection.server.name, selection.reason
            ),
        )
        .await;
    let _ = app.emit(
        crate::commands::servers::AUTO_SELECT_PROGRESS_EVENT,
        crate::commands::servers::AutoSelectProgress {
            stage: "connect".to_string(),
            message: format!("Подключаюсь к {}...", selection.server.name),
        },
    );

    let result = connect_with_state(selection.server, &state).await;
    match &result {
        Ok(message) => {
            let _ = app.emit(
                crate::commands::servers::AUTO_SELECT_PROGRESS_EVENT,
                crate::commands::servers::AutoSelectProgress {
                    stage: "done".to_string(),
                    message: message.clone(),
                },
            );
        }
        Err(error) => {
            let _ = app.emit(
                crate::commands::servers::AUTO_SELECT_PROGRESS_EVENT,
                crate::commands::servers::AutoSelectProgress {
                    stage: "error".to_string(),
                    message: error.clone(),
                },
            );
        }
    }
    let _ = crate::core::tray::refresh_tray_async(app).await;
    result
}

pub async fn connect_with_state(server: Server, state: &AppState) -> Result<String, String> {
    state
        .logs
        .add(
            "info",
            &format!(
                "Подключение к {} ({}:{})...",
                server.name, server.address, server.port
            ),
        )
        .await;
    state
        .logs
        .add(
            "info",
            &format!(
                "Протокол: {:?}, Сеть: {}, Безопасность: {}",
                server.protocol, server.network, server.security
            ),
        )
        .await;

    let settings = state.settings.lock().await.clone();
    state
        .logs
        .add(
            "info",
            &format!(
                "SOCKS: {}, HTTP: {}, TUN: {}",
                settings.proxy.socks_port,
                settings.proxy.http_port,
                if settings.proxy.tun_mode {
                    "ВКЛ"
                } else {
                    "ВЫКЛ"
                }
            ),
        )
        .await;

    // Start xray-core
    match state.xray.start(&server, &settings).await {
        Ok(()) => state.logs.add("success", "Xray-core запущен").await,
        Err(e) => {
            state
                .logs
                .add("error", &format!("Xray ошибка: {}", e))
                .await;
            return Err(format!("Не удалось подключиться: {}", e));
        }
    }

    // Start TUN or system proxy
    if settings.proxy.tun_mode {
        state.logs.add("info", "Запуск TUN режима...").await;
        match state
            .tun
            .start(settings.proxy.socks_port, &server.address)
            .await
        {
            Ok(()) => {
                state
                    .logs
                    .add(
                        "success",
                        "TUN режим активирован — весь трафик идёт через VPN",
                    )
                    .await
            }
            Err(e) => {
                state.logs.add("error", &format!("TUN ошибка: {}", e)).await;
                // Fall back to system proxy
                state.logs.add("warn", "Откат на системный прокси...").await;
                if let Err(pe) = crate::core::proxy::set_system_proxy(
                    settings.proxy.http_port,
                    settings.proxy.socks_port,
                ) {
                    state.logs.add("warn", &format!("Прокси: {}", pe)).await;
                }
            }
        }
    } else if settings.proxy.system_proxy {
        match crate::core::proxy::set_system_proxy(
            settings.proxy.http_port,
            settings.proxy.socks_port,
        ) {
            Ok(()) => state.logs.add("success", "Системный прокси настроен").await,
            Err(e) => state.logs.add("warn", &format!("Прокси: {}", e)).await,
        }
    }

    let mut current = state.current_server.lock().await;
    *current = Some(server.clone());

    state
        .logs
        .add("success", &format!("Подключено к {}", server.name))
        .await;
    Ok(format!("Подключено к {}", server.name))
}

pub async fn disconnect_with_state(state: &AppState) -> Result<String, String> {
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
        state
            .logs
            .add("warn", &format!("Прокси сброс: {}", e))
            .await;
    } else {
        state.logs.add("info", "Системный прокси сброшен").await;
    }

    let mut current = state.current_server.lock().await;
    *current = None;

    state.logs.add("info", "Отключено").await;
    Ok("Отключено".into())
}

#[tauri::command]
pub async fn connect(
    server: Server,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<String, String> {
    let result = connect_with_state(server, &state).await;
    let _ = crate::core::tray::refresh_tray_async(&app).await;
    result
}

#[tauri::command]
pub async fn connect_best_server(app: tauri::AppHandle) -> Result<String, String> {
    connect_best_server_with_app(&app).await
}

#[tauri::command]
pub async fn disconnect(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<String, String> {
    let result = disconnect_with_state(&state).await;
    let _ = crate::core::tray::refresh_tray_async(&app).await;
    result
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
