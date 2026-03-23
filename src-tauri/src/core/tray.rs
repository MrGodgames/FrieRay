use crate::commands::connection::{
    connect_best_server_with_app, connect_with_state, disconnect_with_state,
};
use crate::models::server::Server;
use crate::utils::storage;
use crate::AppState;
use tauri::image::Image;
use tauri::menu::{CheckMenuItem, Menu, MenuBuilder, MenuItemBuilder, SubmenuBuilder};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{
    ActivationPolicy, App, AppHandle, Manager, PhysicalPosition, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder,
};

const TRAY_ID: &str = "main-tray";
const TRAY_POPUP_LABEL: &str = "tray-popup";
const MENU_STATUS: &str = "tray-status";
const MENU_CURRENT: &str = "tray-current";
const MENU_CONNECT: &str = "tray-connect";
const MENU_DISCONNECT: &str = "tray-disconnect";
const MENU_TOGGLE_WINDOW: &str = "tray-toggle-window";
const MENU_QUIT: &str = "tray-quit";
const MENU_SERVER_PREFIX: &str = "tray-server:";
const TRAY_POPUP_WIDTH: f64 = 360.0;
const TRAY_POPUP_HEIGHT: f64 = 440.0;
const TRAY_POPUP_MARGIN: f64 = 10.0;
const TRAY_ICON: &[u8] = include_bytes!("../../icons/tray/dreamsvg-icon.png");

#[derive(Clone)]
struct TraySnapshot {
    connected: bool,
    current_server: Option<Server>,
    active_server: Option<Server>,
    servers: Vec<Server>,
    window_hidden: bool,
}

pub fn setup(app: &mut App) -> tauri::Result<()> {
    let app_handle = app.handle().clone();
    let snapshot = collect_tray_snapshot(&app_handle);
    let menu = build_tray_menu(&app_handle, &snapshot)?;

    let mut tray_builder = TrayIconBuilder::with_id(TRAY_ID)
        .menu(&menu)
        .tooltip(build_tray_tooltip(&snapshot))
        .icon_as_template(false)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| {
            handle_menu_event(app, event.id().as_ref().to_string());
        })
        .on_tray_icon_event(|tray, event| {
            handle_tray_icon_event(tray.app_handle(), event);
        });

    if let Some(icon) = tray_icon_image(snapshot.connected) {
        tray_builder = tray_builder.icon(icon);
    }

    tray_builder.build(app)?;
    let _ = ensure_tray_popup_window(&app_handle);
    Ok(())
}

pub fn refresh_tray(app: &AppHandle) -> Result<(), String> {
    let snapshot = tauri::async_runtime::block_on(collect_tray_snapshot_async(app));
    refresh_tray_with_snapshot(app, &snapshot)
}

pub async fn refresh_tray_async(app: &AppHandle) -> Result<(), String> {
    let snapshot = collect_tray_snapshot_async(app).await;
    refresh_tray_with_snapshot(app, &snapshot)
}

fn refresh_tray_with_snapshot(app: &AppHandle, snapshot: &TraySnapshot) -> Result<(), String> {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return Ok(());
    };

    let menu = build_tray_menu(app, snapshot).map_err(|e| e.to_string())?;
    tray.set_menu(Some(menu)).map_err(|e| e.to_string())?;
    tray.set_tooltip(Some(build_tray_tooltip(snapshot)))
        .map_err(|e| e.to_string())?;
    tray.set_title(None::<String>).map_err(|e| e.to_string())?;
    tray.set_icon_as_template(false)
        .map_err(|e| e.to_string())?;
    if let Some(icon) = tray_icon_image(snapshot.connected) {
        tray.set_icon(Some(icon)).map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn show_main_window(app: &AppHandle) -> Result<(), String> {
    hide_tray_popup(app);

    #[cfg(target_os = "macos")]
    app.set_activation_policy(ActivationPolicy::Regular)
        .map_err(|e| e.to_string())?;

    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "Главное окно не найдено".to_string())?;
    let _ = window.show();
    let _ = window.unminimize();
    let _ = window.set_focus();
    refresh_tray(app)?;
    Ok(())
}

pub fn hide_main_window(app: &AppHandle) -> Result<(), String> {
    hide_tray_popup(app);

    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "Главное окно не найдено".to_string())?;
    let _ = window.hide();

    #[cfg(target_os = "macos")]
    app.set_activation_policy(ActivationPolicy::Accessory)
        .map_err(|e| e.to_string())?;

    refresh_tray(app)?;
    Ok(())
}

pub fn apply_startup_behavior(app: &AppHandle) {
    let settings = {
        let state = app.state::<AppState>();
        tauri::async_runtime::block_on(async { state.settings.lock().await.clone() })
    };

    if settings.general.start_minimized {
        let _ = hide_main_window(app);
    } else {
        #[cfg(target_os = "macos")]
        let _ = app.set_activation_policy(ActivationPolicy::Regular);
    }

    if settings.general.auto_connect {
        let app_handle = app.clone();
        tauri::async_runtime::spawn(async move {
            let server = {
                let state = app_handle.state::<AppState>();
                let server = state.active_server.lock().await.clone();
                server
            };

            if let Some(server) = server {
                let state = app_handle.state::<AppState>();
                if let Err(error) = connect_with_state(server, &state).await {
                    state
                        .logs
                        .add("error", &format!("Автоподключение: {}", error))
                        .await;
                }
                let _ = refresh_tray_async(&app_handle).await;
            }
        });
    } else {
        let _ = refresh_tray(app);
    }
}

fn tray_icon_image(connected: bool) -> Option<Image<'static>> {
    let base = Image::from_bytes(TRAY_ICON).ok()?;
    let mut rgba = base.rgba().to_vec();

    if !connected {
        for pixel in rgba.chunks_exact_mut(4) {
            pixel[3] = ((pixel[3] as u16 * 140) / 255) as u8;
        }
    }

    Some(Image::new_owned(rgba, base.width(), base.height()))
}

fn handle_menu_event(app: &AppHandle, id: String) {
    match id.as_str() {
        MENU_TOGGLE_WINDOW => {
            let snapshot = collect_tray_snapshot(app);
            let result = if snapshot.window_hidden {
                show_main_window(app)
            } else {
                hide_main_window(app)
            };
            if let Err(error) = result {
                log::warn!("Tray window toggle: {}", error);
            }
        }
        MENU_QUIT => {
            app.exit(0);
        }
        MENU_CONNECT => {
            let app_handle = app.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(error) = connect_best_server_with_app(&app_handle).await {
                    let state = app_handle.state::<AppState>();
                    state
                        .logs
                        .add("error", &format!("Tray connect: {}", error))
                        .await;
                }
                let _ = refresh_tray_async(&app_handle).await;
            });
        }
        MENU_DISCONNECT => {
            let app_handle = app.clone();
            tauri::async_runtime::spawn(async move {
                let state = app_handle.state::<AppState>();
                if let Err(error) = disconnect_with_state(&state).await {
                    state
                        .logs
                        .add("error", &format!("Tray disconnect: {}", error))
                        .await;
                }
                let _ = refresh_tray_async(&app_handle).await;
            });
        }
        _ if id.starts_with(MENU_SERVER_PREFIX) => {
            let server_id = id.trim_start_matches(MENU_SERVER_PREFIX).to_string();
            let app_handle = app.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(error) = select_server_from_tray(&app_handle, &server_id).await {
                    let state = app_handle.state::<AppState>();
                    state
                        .logs
                        .add("error", &format!("Tray server switch: {}", error))
                        .await;
                }
                let _ = refresh_tray_async(&app_handle).await;
            });
        }
        _ => {}
    }
}

fn handle_tray_icon_event(app: &AppHandle, event: TrayIconEvent) {
    if let TrayIconEvent::Click {
        button,
        button_state: MouseButtonState::Up,
        position,
        ..
    } = event
    {
        match button {
            MouseButton::Left => {
                let _ = toggle_tray_popup(app, position);
                let _ = refresh_tray(app);
            }
            MouseButton::Right => {
                hide_tray_popup(app);
                let _ = refresh_tray(app);
            }
            _ => {}
        }
    }
}

fn ensure_tray_popup_window(app: &AppHandle) -> Result<(), String> {
    if app.get_webview_window(TRAY_POPUP_LABEL).is_some() {
        return Ok(());
    }

    WebviewWindowBuilder::new(app, TRAY_POPUP_LABEL, WebviewUrl::default())
        .title("FrieRay Tray")
        .inner_size(TRAY_POPUP_WIDTH, TRAY_POPUP_HEIGHT)
        .min_inner_size(TRAY_POPUP_WIDTH, TRAY_POPUP_HEIGHT)
        .max_inner_size(TRAY_POPUP_WIDTH, TRAY_POPUP_HEIGHT)
        .resizable(false)
        .visible(false)
        .focused(false)
        .decorations(false)
        .skip_taskbar(true)
        .always_on_top(true)
        .visible_on_all_workspaces(true)
        .shadow(true)
        .build()
        .map_err(|e| e.to_string())?;

    Ok(())
}

fn toggle_tray_popup(app: &AppHandle, position: PhysicalPosition<f64>) -> Result<(), String> {
    ensure_tray_popup_window(app)?;
    let popup = app
        .get_webview_window(TRAY_POPUP_LABEL)
        .ok_or_else(|| "Tray popup не найден".to_string())?;

    if popup.is_visible().unwrap_or(false) && popup.is_focused().unwrap_or(false) {
        popup.hide().map_err(|e| e.to_string())?;
        return Ok(());
    }

    position_tray_popup(app, &popup, position)?;
    popup.show().map_err(|e| e.to_string())?;
    popup.set_focus().map_err(|e| e.to_string())?;
    Ok(())
}

fn hide_tray_popup(app: &AppHandle) {
    if let Some(popup) = app.get_webview_window(TRAY_POPUP_LABEL) {
        let _ = popup.hide();
    }
}

fn position_tray_popup(
    app: &AppHandle,
    popup: &WebviewWindow,
    position: PhysicalPosition<f64>,
) -> Result<(), String> {
    let mut x = position.x - (TRAY_POPUP_WIDTH / 2.0);
    let mut y = position.y + TRAY_POPUP_MARGIN;

    if let Some(bounds) = monitor_bounds_for_point(app, position) {
        let min_x = bounds.0;
        let max_x = bounds.0 + bounds.2 - TRAY_POPUP_WIDTH;
        let max_y = bounds.1 + bounds.3 - TRAY_POPUP_HEIGHT - TRAY_POPUP_MARGIN;
        x = x.clamp(min_x, max_x.max(min_x));
        y = y.clamp(
            bounds.1 + TRAY_POPUP_MARGIN,
            max_y.max(bounds.1 + TRAY_POPUP_MARGIN),
        );
    }

    popup
        .set_position(PhysicalPosition::new(x.round() as i32, y.round() as i32))
        .map_err(|e| e.to_string())
}

fn monitor_bounds_for_point(
    app: &AppHandle,
    point: PhysicalPosition<f64>,
) -> Option<(f64, f64, f64, f64)> {
    let probe = app
        .get_webview_window(TRAY_POPUP_LABEL)
        .or_else(|| app.get_webview_window("main"))?;
    let monitors = probe.available_monitors().ok()?;
    for monitor in monitors {
        let work_area = monitor.work_area();
        let x = work_area.position.x as f64;
        let y = work_area.position.y as f64;
        let width = work_area.size.width as f64;
        let height = work_area.size.height as f64;
        if point.x >= x && point.x <= x + width && point.y >= y && point.y <= y + height {
            return Some((x, y, width, height));
        }
    }
    None
}

async fn select_server_from_tray(app: &AppHandle, server_id: &str) -> Result<(), String> {
    let state = app.state::<AppState>();
    let server = {
        let servers = state.servers.lock().await;
        servers
            .iter()
            .find(|server| server.id == server_id)
            .cloned()
            .ok_or_else(|| "Сервер не найден".to_string())?
    };

    {
        let mut active = state.active_server.lock().await;
        *active = Some(server.clone());
    }
    storage::save_active_server_id(&server.id)?;

    if state.xray.is_running().await {
        disconnect_with_state(&state).await?;
        connect_with_state(server.clone(), &state).await?;
    } else {
        state
            .logs
            .add(
                "info",
                &format!("Активный сервер изменён на {}", server.name),
            )
            .await;
    }

    Ok(())
}

fn collect_tray_snapshot(app: &AppHandle) -> TraySnapshot {
    tauri::async_runtime::block_on(collect_tray_snapshot_async(app))
}

async fn collect_tray_snapshot_async(app: &AppHandle) -> TraySnapshot {
    let state = app.state::<AppState>();
    let connected = state.xray.is_running().await;
    let current_server = state.current_server.lock().await.clone();
    let active_server = state.active_server.lock().await.clone();
    let servers = state.servers.lock().await.clone();
    let window_hidden = app
        .get_webview_window("main")
        .and_then(|window| {
            let is_visible = window.is_visible().ok()?;
            let is_minimized = window.is_minimized().ok().unwrap_or(false);
            Some(!is_visible || is_minimized)
        })
        .unwrap_or(false);

    TraySnapshot {
        connected,
        current_server,
        active_server,
        servers,
        window_hidden,
    }
}

fn build_tray_menu(app: &AppHandle, snapshot: &TraySnapshot) -> tauri::Result<Menu<tauri::Wry>> {
    let status_text = if snapshot.connected {
        "Статус: подключено"
    } else {
        "Статус: не подключено"
    };
    let current_text = match snapshot
        .current_server
        .as_ref()
        .or(snapshot.active_server.as_ref())
    {
        Some(server) => format!("Сервер: {}", server.name),
        None => "Сервер: не выбран".to_string(),
    };

    let mut server_list = snapshot.servers.clone();
    let active_server_id = snapshot
        .active_server
        .as_ref()
        .map(|server| server.id.as_str());
    server_list.sort_by(|left, right| {
        let left_active = Some(left.id.as_str()) == active_server_id;
        let right_active = Some(right.id.as_str()) == active_server_id;
        right_active
            .cmp(&left_active)
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
    });

    let mut server_submenu_builder = SubmenuBuilder::new(app, "Серверы");
    if server_list.is_empty() {
        let empty_item = MenuItemBuilder::with_id("tray-no-servers", "Нет серверов")
            .enabled(false)
            .build(app)?;
        server_submenu_builder = server_submenu_builder.item(&empty_item);
    } else {
        for server in &server_list {
            let server_item = CheckMenuItem::with_id(
                app,
                format!("{MENU_SERVER_PREFIX}{}", server.id),
                &server.name,
                true,
                Some(server.id.as_str()) == active_server_id,
                None::<&str>,
            )?;
            server_submenu_builder = server_submenu_builder.item(&server_item);
        }
    }
    let servers_submenu = server_submenu_builder.build()?;

    let status_item = MenuItemBuilder::with_id(MENU_STATUS, status_text)
        .enabled(false)
        .build(app)?;
    let current_item = MenuItemBuilder::with_id(MENU_CURRENT, current_text)
        .enabled(false)
        .build(app)?;
    let connect_item = MenuItemBuilder::with_id(MENU_CONNECT, "Подключить лучший")
        .enabled(!snapshot.connected && !server_list.is_empty())
        .build(app)?;
    let disconnect_item = MenuItemBuilder::with_id(MENU_DISCONNECT, "Отключить")
        .enabled(snapshot.connected)
        .build(app)?;
    let window_toggle_text = if snapshot.window_hidden {
        "Показать окно"
    } else {
        "Скрыть окно"
    };
    let window_toggle_item =
        MenuItemBuilder::with_id(MENU_TOGGLE_WINDOW, window_toggle_text).build(app)?;
    let quit_item = MenuItemBuilder::with_id(MENU_QUIT, "Выйти из FrieRay").build(app)?;

    MenuBuilder::new(app)
        .item(&status_item)
        .item(&current_item)
        .separator()
        .item(&connect_item)
        .item(&disconnect_item)
        .separator()
        .item(&window_toggle_item)
        .item(&servers_submenu)
        .separator()
        .item(&quit_item)
        .build()
}

fn build_tray_tooltip(snapshot: &TraySnapshot) -> String {
    if snapshot.connected {
        match snapshot
            .current_server
            .as_ref()
            .or(snapshot.active_server.as_ref())
        {
            Some(server) => format!("FrieRay — подключено к {}", server.name),
            None => "FrieRay — подключено".to_string(),
        }
    } else {
        match snapshot.active_server.as_ref() {
            Some(server) => format!("FrieRay — готов к подключению: {}", server.name),
            None => "FrieRay — не подключено".to_string(),
        }
    }
}
