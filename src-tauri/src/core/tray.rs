use crate::commands::connection::{connect_with_state, reconnect_best_server_rescan_with_app};
use crate::commands::servers::ping_server;
use crate::models::server::Server;
use crate::AppState;
use tauri::image::Image;
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{
    ActivationPolicy, App, AppHandle, Manager, PhysicalPosition, Rect, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder,
};

const TRAY_ID: &str = "main-tray";
const TRAY_POPUP_LABEL: &str = "tray-popup";
const TRAY_POPUP_WIDTH: f64 = 360.0;
const TRAY_POPUP_HEIGHT: f64 = 320.0;
const TRAY_POPUP_MARGIN: f64 = 10.0;
const HEALTH_CHECK_INTERVAL_SECS: u64 = 15;
const HEALTH_CHECK_FAILURES_BEFORE_FAILOVER: u8 = 2;
const TRAY_ICON: &[u8] = include_bytes!("../../icons/tray/dreamsvg-icon.png");

#[derive(Clone)]
struct TraySnapshot {
    connected: bool,
    current_server: Option<Server>,
    active_server: Option<Server>,
}

pub fn setup(app: &mut App) -> tauri::Result<()> {
    let app_handle = app.handle().clone();
    let snapshot = collect_tray_snapshot(&app_handle);

    let mut tray_builder = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip(build_tray_tooltip(&snapshot))
        .icon_as_template(false)
        .show_menu_on_left_click(false)
        .on_tray_icon_event(|tray, event| {
            handle_tray_icon_event(tray.app_handle(), event);
        });

    if let Some(icon) = tray_icon_image(snapshot.connected) {
        tray_builder = tray_builder.icon(icon);
    }

    tray_builder.build(app)?;
    start_connection_health_monitor(&app_handle);
    Ok(())
}

fn start_connection_health_monitor(app: &AppHandle) {
    let app_handle = app.clone();
    tauri::async_runtime::spawn(async move {
        let mut failures = 0u8;

        loop {
            tokio::time::sleep(std::time::Duration::from_secs(HEALTH_CHECK_INTERVAL_SECS)).await;

            let state = app_handle.state::<AppState>();
            if !state.xray.is_running().await {
                failures = 0;
                continue;
            }

            let server = state.current_server.lock().await.clone();
            let Some(server) = server else {
                failures = 0;
                continue;
            };

            match ping_server(server.address.clone(), server.port).await {
                Ok(_) => {
                    failures = 0;
                }
                Err(error) => {
                    failures = failures.saturating_add(1);
                    state
                        .logs
                        .add(
                            "warn",
                            &format!(
                                "Tray health check: {} не отвечает ({}/{}): {}",
                                server.name, failures, HEALTH_CHECK_FAILURES_BEFORE_FAILOVER, error
                            ),
                        )
                        .await;

                    if failures >= HEALTH_CHECK_FAILURES_BEFORE_FAILOVER {
                        state
                            .logs
                            .add(
                                "warn",
                                "Tray health check: текущий сервер недоступен, запускаю автопереподключение...",
                            )
                            .await;

                        if let Err(error) = reconnect_best_server_rescan_with_app(&app_handle).await
                        {
                            let state = app_handle.state::<AppState>();
                            state
                                .logs
                                .add("error", &format!("Tray auto failover: {}", error))
                                .await;
                        }
                        let _ = refresh_tray_async(&app_handle).await;
                        failures = 0;
                    }
                }
            }
        }
    });
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

    // Keep the native tray menu disabled. FrieRay uses a custom webview popup so
    // the menu-bar interaction feels like the app UI instead of falling back to
    // the default macOS context menu on some machines/right-clicks.
    tray.set_menu(None::<tauri::menu::Menu<tauri::Wry>>)
        .map_err(|e| e.to_string())?;
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

fn handle_tray_icon_event(app: &AppHandle, event: TrayIconEvent) {
    if let TrayIconEvent::Click {
        button,
        button_state: MouseButtonState::Up,
        position,
        rect,
        ..
    } = event
    {
        match button {
            MouseButton::Left | MouseButton::Right => {
                let _ = toggle_tray_popup(app, position, rect);
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
        .transparent(true)
        .shadow(false)
        .build()
        .map_err(|e| e.to_string())?;

    Ok(())
}

fn toggle_tray_popup(
    app: &AppHandle,
    position: PhysicalPosition<f64>,
    rect: Rect,
) -> Result<(), String> {
    ensure_tray_popup_window(app)?;
    let popup = app
        .get_webview_window(TRAY_POPUP_LABEL)
        .ok_or_else(|| "Tray popup не найден".to_string())?;

    if popup.is_visible().unwrap_or(false) && popup.is_focused().unwrap_or(false) {
        popup.hide().map_err(|e| e.to_string())?;
        return Ok(());
    }

    position_tray_popup(app, &popup, position, rect)?;
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
    rect: Rect,
) -> Result<(), String> {
    let anchor = tray_anchor_point(position, rect);
    let mut x = anchor.x - (TRAY_POPUP_WIDTH / 2.0);
    let mut y = anchor.y + TRAY_POPUP_MARGIN;

    if let Some(bounds) =
        monitor_bounds_for_point(app, anchor).or_else(|| primary_monitor_bounds(app))
    {
        let min_x = bounds.0;
        let max_x = bounds.0 + bounds.2 - TRAY_POPUP_WIDTH;
        let min_y = bounds.1 + TRAY_POPUP_MARGIN;
        let max_y = bounds.1 + bounds.3 - TRAY_POPUP_HEIGHT - TRAY_POPUP_MARGIN;
        let screen_mid_y = bounds.1 + bounds.3 / 2.0;
        y = if anchor.y > screen_mid_y {
            anchor.y - TRAY_POPUP_HEIGHT - TRAY_POPUP_MARGIN
        } else {
            anchor.y + TRAY_POPUP_MARGIN
        };
        x = x.clamp(min_x, max_x.max(min_x));
        y = y.clamp(min_y, max_y.max(min_y));
    }

    popup
        .set_position(PhysicalPosition::new(x.round() as i32, y.round() as i32))
        .map_err(|e| e.to_string())
}

fn tray_anchor_point(position: PhysicalPosition<f64>, rect: Rect) -> PhysicalPosition<f64> {
    let (rect_width, rect_height) = match rect.size {
        tauri::Size::Physical(size) => (size.width as f64, size.height as f64),
        tauri::Size::Logical(size) => (size.width, size.height),
    };
    let (rect_x, rect_y) = match rect.position {
        tauri::Position::Physical(position) => (position.x as f64, position.y as f64),
        tauri::Position::Logical(position) => (position.x, position.y),
    };
    if rect_width > 0.0 && rect_height > 0.0 {
        return PhysicalPosition::new(rect_x + rect_width / 2.0, rect_y + rect_height);
    }
    position
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

fn primary_monitor_bounds(app: &AppHandle) -> Option<(f64, f64, f64, f64)> {
    let probe = app
        .get_webview_window(TRAY_POPUP_LABEL)
        .or_else(|| app.get_webview_window("main"))?;
    let monitor = probe.primary_monitor().ok().flatten()?;
    let work_area = monitor.work_area();
    Some((
        work_area.position.x as f64,
        work_area.position.y as f64,
        work_area.size.width as f64,
        work_area.size.height as f64,
    ))
}

fn collect_tray_snapshot(app: &AppHandle) -> TraySnapshot {
    tauri::async_runtime::block_on(collect_tray_snapshot_async(app))
}

async fn collect_tray_snapshot_async(app: &AppHandle) -> TraySnapshot {
    let state = app.state::<AppState>();
    let connected = state.xray.is_running().await;
    let current_server = state.current_server.lock().await.clone();
    let active_server = state.active_server.lock().await.clone();

    TraySnapshot {
        connected,
        current_server,
        active_server,
    }
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
