use crate::models::server::Server;
use crate::AppState;
use tauri::{Emitter, Manager, State};

const CONNECTIVITY_PROBE_TARGETS: &[(&str, &str)] = &[
    ("Selectel", "https://speedtest.selectel.ru/100MB"),
    ("Yandex Mirror", "https://mirror.yandex.ru/debian/ls-lR.gz"),
];
const CONNECTIVITY_PROBE_BYTES: u64 = 16 * 1024;
const CONNECTIVITY_PROBE_TIMEOUT_SECS: u64 = 8;

pub async fn connect_best_server_with_app(app: &tauri::AppHandle) -> Result<String, String> {
    connect_ranked_server_with_app(app, false, false).await
}

pub async fn connect_best_server_rescan_with_app(app: &tauri::AppHandle) -> Result<String, String> {
    connect_ranked_server_with_app(app, true, false).await
}

pub async fn reconnect_best_server_rescan_with_app(
    app: &tauri::AppHandle,
) -> Result<String, String> {
    connect_ranked_server_with_app(app, true, true).await
}

async fn connect_ranked_server_with_app(
    app: &tauri::AppHandle,
    force_rescan: bool,
    exclude_current: bool,
) -> Result<String, String> {
    let state = app.state::<AppState>();
    let current_id = if exclude_current {
        state
            .current_server
            .lock()
            .await
            .as_ref()
            .map(|server| server.id.clone())
    } else {
        None
    };
    let excluded_ids = current_id.into_iter().collect::<Vec<_>>();
    let candidates = crate::commands::servers::rank_servers_for_auto_select(
        app,
        &state,
        force_rescan,
        &excluded_ids,
    )
    .await?;

    if state.xray.is_running().await {
        let _ = app.emit(
            crate::commands::servers::AUTO_SELECT_PROGRESS_EVENT,
            crate::commands::servers::AutoSelectProgress {
                stage: "disconnect".to_string(),
                message: "Отключаю текущее соединение перед сменой сервера...".into(),
            },
        );
        let _ = disconnect_with_state(&state).await;
    }

    let mut last_error = None;

    for server in candidates {
        let reason = if let Some(speed) = server.speed_mbps {
            format!("по скорости {:.1} Mb/s", speed)
        } else if let Some(ping) = server.ping {
            format!("по ping {} ms", ping)
        } else {
            "по доступности".into()
        };

        {
            let mut active = state.active_server.lock().await;
            *active = Some(server.clone());
        }
        crate::utils::storage::save_active_server_id(&server.id)?;

        state
            .logs
            .add(
                "info",
                &format!("Автовыбор сервера: {} {}", server.name, reason),
            )
            .await;
        let _ = app.emit(
            crate::commands::servers::AUTO_SELECT_PROGRESS_EVENT,
            crate::commands::servers::AutoSelectProgress {
                stage: "connect".to_string(),
                message: format!("Подключаюсь к {}...", server.name),
            },
        );

        match connect_with_state(server.clone(), &state).await {
            Ok(message) => {
                let _ = app.emit(
                    crate::commands::servers::AUTO_SELECT_PROGRESS_EVENT,
                    crate::commands::servers::AutoSelectProgress {
                        stage: "done".to_string(),
                        message: message.clone(),
                    },
                );
                let _ = crate::core::tray::refresh_tray_async(app).await;
                return Ok(message);
            }
            Err(error) => {
                state
                    .logs
                    .add(
                        "warn",
                        &format!("{} не прошёл подключение, пробую следующий сервер", server.name),
                    )
                    .await;
                let _ = app.emit(
                    crate::commands::servers::AUTO_SELECT_PROGRESS_EVENT,
                    crate::commands::servers::AutoSelectProgress {
                        stage: "retry".to_string(),
                        message: format!("{} не ответил, пробую следующий...", server.name),
                    },
                );
                let _ = disconnect_with_state(&state).await;
                last_error = Some(error);
            }
        }
    }

    let error = last_error.unwrap_or_else(|| "Нет доступных серверов для подключения".into());
    let _ = app.emit(
        crate::commands::servers::AUTO_SELECT_PROGRESS_EVENT,
        crate::commands::servers::AutoSelectProgress {
            stage: "error".to_string(),
            message: error.clone(),
        },
    );
    let _ = crate::core::tray::refresh_tray_async(app).await;
    Err(error)
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

    if let Err(e) = verify_local_proxy_connectivity(settings.proxy.socks_port).await {
        let _ = state.xray.stop().await;
        state
            .logs
            .add(
                "error",
                &format!("Прокси запущен, но сервер не пропускает трафик: {}", e),
            )
            .await;
        return Err(format!(
            "Сервер не прошёл проверку подключения через Xray: {}",
            e
        ));
    }
    state
        .logs
        .add("success", "Проверка трафика через Xray прошла")
        .await;

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
                    let _ = state.xray.stop().await;
                    return Err(format!(
                        "TUN не запустился, а системный прокси не удалось включить: {}",
                        pe
                    ));
                }
            }
        }
    } else if settings.proxy.system_proxy {
        match crate::core::proxy::set_system_proxy(
            settings.proxy.http_port,
            settings.proxy.socks_port,
        ) {
            Ok(()) => state.logs.add("success", "Системный прокси настроен").await,
            Err(e) => {
                state.logs.add("error", &format!("Прокси: {}", e)).await;
                let _ = state.xray.stop().await;
                return Err(format!("Не удалось включить системный прокси: {}", e));
            }
        }
    } else {
        state
            .logs
            .add(
                "warn",
                "TUN и системный прокси выключены — доступен только локальный SOCKS/HTTP proxy",
            )
            .await;
    }

    let mut current = state.current_server.lock().await;
    *current = Some(server.clone());

    state
        .logs
        .add("success", &format!("Подключено к {}", server.name))
        .await;
    Ok(format!("Подключено к {}", server.name))
}

async fn verify_local_proxy_connectivity(socks_port: u16) -> Result<(), String> {
    let proxy = reqwest::Proxy::all(format!("socks5h://127.0.0.1:{}", socks_port))
        .map_err(|e| format!("Proxy error: {}", e))?;
    let client = reqwest::Client::builder()
        .proxy(proxy)
        .connect_timeout(std::time::Duration::from_secs(3))
        .timeout(std::time::Duration::from_secs(
            CONNECTIVITY_PROBE_TIMEOUT_SECS,
        ))
        .build()
        .map_err(|e| format!("Client error: {}", e))?;

    let mut errors = Vec::new();
    for (name, url) in CONNECTIVITY_PROBE_TARGETS {
        match run_connectivity_probe(&client, url).await {
            Ok(()) => return Ok(()),
            Err(e) => errors.push(format!("{}: {}", name, e)),
        }
    }

    Err(errors.join(" | "))
}

async fn run_connectivity_probe(client: &reqwest::Client, url: &str) -> Result<(), String> {
    let response = client
        .get(url)
        .header(
            reqwest::header::RANGE,
            format!("bytes=0-{}", CONNECTIVITY_PROBE_BYTES - 1),
        )
        .send()
        .await
        .map_err(|e| format!("request error for {}: {}", url, e))?
        .error_for_status()
        .map_err(|e| format!("HTTP error for {}: {}", url, e))?;

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("read error for {}: {}", url, e))?;

    if bytes.is_empty() {
        Err(format!("{} returned empty response", url))
    } else {
        Ok(())
    }
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
pub async fn connect_best_server_rescan(app: tauri::AppHandle) -> Result<String, String> {
    connect_best_server_rescan_with_app(&app).await
}

#[tauri::command]
pub async fn reconnect_best_server_rescan(app: tauri::AppHandle) -> Result<String, String> {
    reconnect_best_server_rescan_with_app(&app).await
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
