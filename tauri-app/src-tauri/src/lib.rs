mod anomaly;
mod backup;
mod commands;
mod export;
mod insights;
mod metrics;
mod pricing;
mod prometheus;
mod prometheus_health;
mod sessions;
mod stack;
mod telemetry;
mod tray;
mod usage;

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, PhysicalPosition, Rect, State,
};

use tray::TrayState;

/// Show the main window (and the Dock icon on macOS).
fn show_main(app: &tauri::AppHandle) {
    #[cfg(target_os = "macos")]
    let _ = app.set_activation_policy(tauri::ActivationPolicy::Regular);
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

/// Hide the main window; on macOS also drop the Dock icon so the app lives only in the menu bar.
fn hide_main(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.hide();
    }
    #[cfg(target_os = "macos")]
    let _ = app.set_activation_policy(tauri::ActivationPolicy::Accessory);
}

/// When the popover last lost focus (ms since epoch). Clicking the tray icon while the popover is open blurs it first,
/// so without this the same click would hide it and immediately re-open it.
static POPOVER_HIDDEN_AT: AtomicU64 = AtomicU64::new(0);

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

fn hide_popover(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("popover") {
        let _ = w.hide();
    }
    POPOVER_HIDDEN_AT.store(now_ms(), Ordering::Relaxed);
}

/// While true the popover behaves like a desktop widget: it stays visible (and on top) when it loses focus.
static POPOVER_PINNED: AtomicBool = AtomicBool::new(false);

/// Screen rectangle of the tray icon, if the OS reports one yet.
fn tray_rect(app: &tauri::AppHandle) -> Option<Rect> {
    let state: State<TrayState> = app.state();
    let guard = state.tray.lock().ok()?;
    guard.as_ref()?.rect().ok().flatten()
}

/// True when the centre of the tray rectangle lies on one of the connected monitors. With a crowded menu bar macOS
/// pushes overflowing status items off-screen (negative y), and a widget anchored there would be invisible.
fn rect_on_screen(app: &tauri::AppHandle, rect: &Rect, scale: f64) -> bool {
    let p = rect.position.to_physical::<f64>(scale);
    let s = rect.size.to_physical::<f64>(scale);
    let (cx, cy) = (p.x + s.width / 2.0, p.y + s.height / 2.0);
    app.available_monitors().unwrap_or_default().iter().any(|m| {
        let (mx, my) = (m.position().x as f64, m.position().y as f64);
        cx >= mx && cx < mx + m.size().width as f64 && cy >= my && cy < my + m.size().height as f64
    })
}

/// Put the popover under the tray icon (`rect`); without a usable rect (none, or off-screen), top-right of the primary monitor.
fn place_popover(app: &tauri::AppHandle, w: &tauri::WebviewWindow, rect: Option<Rect>) {
    let scale = w.scale_factor().unwrap_or(1.0);
    let rect = rect.filter(|r| rect_on_screen(app, r, scale));
    let width = w.outer_size().map(|s| s.width as f64).unwrap_or(320.0 * scale);
    let (x, y) = match rect {
        Some(rect) => {
            let pos = rect.position.to_physical::<f64>(scale);
            let size = rect.size.to_physical::<f64>(scale);
            ((pos.x + size.width / 2.0 - width / 2.0).max(0.0), pos.y + size.height + 4.0 * scale)
        }
        None => match app.primary_monitor().ok().flatten() {
            Some(m) => (
                m.position().x as f64 + m.size().width as f64 - width - 16.0 * scale,
                m.position().y as f64 + 32.0 * scale,
            ),
            None => return, // nothing sensible to anchor to: leave the window where it is
        },
    };
    let _ = w.set_position(PhysicalPosition::new(x as i32, y as i32));
}

/// Toggle the small status window anchored under the tray icon (`rect` is the icon's screen rectangle).
fn toggle_popover(app: &tauri::AppHandle, rect: Rect) {
    let Some(w) = app.get_webview_window("popover") else { return };
    if w.is_visible().unwrap_or(false) {
        hide_popover(app);
        return;
    }
    if now_ms().saturating_sub(POPOVER_HIDDEN_AT.load(Ordering::Relaxed)) < 250 {
        return; // this click is the one that just blurred (and hid) the popover
    }
    place_popover(app, &w, Some(rect));
    let _ = w.show();
    let _ = w.set_focus();
}

/// Comparable form of a tray rectangle (physical pixels at scale 1), used only to detect that it moved.
fn rect_key(rect: &Rect) -> (i64, i64, i64, i64) {
    let p = rect.position.to_physical::<f64>(1.0);
    let s = rect.size.to_physical::<f64>(1.0);
    (p.x as i64, p.y as i64, s.width as i64, s.height as i64)
}

/// Apply the pinned state. Pinning shows the widget immediately and keeps it open until unpinned.
///
/// At launch macOS reports the tray icon at a temporary position (even off-screen) and moves it to its real
/// place several seconds later. So after the first placement keep following the icon until its rectangle has
/// been stable for ~4 s (or 60 s passed); the widget may live on another display than the primary one.
fn apply_pin(app: tauri::AppHandle, pinned: bool) {
    POPOVER_PINNED.store(pinned, Ordering::Relaxed);
    if !pinned {
        return;
    }
    let Some(w) = app.get_webview_window("popover") else { return };
    if w.is_visible().unwrap_or(false) {
        return;
    }
    let initial = tray_rect(&app);
    place_popover(&app, &w, initial);
    let _ = w.show();

    std::thread::spawn(move || {
        let mut last = initial.as_ref().map(rect_key);
        let mut stable = 0;
        for _ in 0..120 {
            std::thread::sleep(std::time::Duration::from_millis(500));
            if !POPOVER_PINNED.load(Ordering::Relaxed) {
                return;
            }
            let cur = tray_rect(&app);
            let key = cur.as_ref().map(rect_key);
            if key != last {
                if let (Some(rect), Some(w)) = (cur, app.get_webview_window("popover")) {
                    place_popover(&app, &w, Some(rect));
                }
                last = key;
                stable = 0;
            } else {
                stable += 1;
                if stable >= 8 && key.is_some() {
                    return;
                }
            }
        }
    });
}

/// The pinned state lives in a small file (not webview localStorage, which may not be flushed if the app is killed).
fn pin_file(app: &tauri::AppHandle) -> Option<std::path::PathBuf> {
    app.path().app_data_dir().ok().map(|d| d.join("widget-pinned"))
}

fn read_pinned(app: &tauri::AppHandle) -> bool {
    pin_file(app).and_then(|p| std::fs::read_to_string(p).ok()).map_or(false, |t| t.trim() == "1")
}

/// Pin/unpin the widget from its button; the choice is persisted and restored by the backend at launch.
#[tauri::command]
fn set_popover_pinned(app: tauri::AppHandle, pinned: bool) {
    if let Some(path) = pin_file(&app) {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(path, if pinned { "1" } else { "0" });
    }
    apply_pin(app, pinned);
}

#[tauri::command]
fn get_popover_pinned(app: tauri::AppHandle) -> bool {
    read_pinned(&app)
}

/// Popover button: open the full dashboard and dismiss the popover.
#[tauri::command]
fn open_dashboard(app: tauri::AppHandle) {
    hide_popover(&app);
    show_main(&app);
}

/// Popover button: quit the whole app.
#[tauri::command]
fn quit_app(app: tauri::AppHandle) {
    app.exit(0);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_notification::init())
        .manage(TrayState::new())
        .setup(|app| {
            #[cfg(desktop)]
            app.handle().plugin(tauri_plugin_autostart::init(
                tauri_plugin_autostart::MacosLauncher::LaunchAgent,
                Some(vec!["--hidden"]), // login launch starts in the menu bar without popping up the window
            ))?;

            if std::env::args().any(|a| a == "--hidden") {
                hide_main(app.handle());
            }

            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let show = MenuItem::with_id(app, "show", "Open Dashboard", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &quit])?;

            let tray_icon = tauri::image::Image::from_bytes(include_bytes!("../icons/tray-template.png"))
                .unwrap_or_else(|_| app.default_window_icon().unwrap().clone());
            let tray = TrayIconBuilder::new()
                .icon(tray_icon)
                .icon_as_template(true) // monochrome, adapts to light/dark menu bar
                .menu(&menu)
                .show_menu_on_left_click(false) // left click opens the status popover; right click shows this menu
                .title("🔴 --") // Initial placeholder (red = not connected yet)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "quit" => {
                        app.exit(0);
                    }
                    "show" => show_main(app),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        rect,
                        ..
                    } = event
                    {
                        toggle_popover(tray.app_handle(), rect);
                    }
                })
                .build(app)?;

            // Store tray handle in state for later updates
            let tray_state: State<TrayState> = app.state();
            if let Ok(mut guard) = tray_state.tray.lock() {
                *guard = Some(tray);
            }
            tray::start_poller(app.handle().clone());
            // Restore the pinned widget (also after login) without waiting for any webview.
            if read_pinned(app.handle()) {
                // Not inside setup itself: windows cannot be shown reliably before the event loop is running.
                let handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_millis(1200)).await;
                    apply_pin(handle, true);
                });
            }

            Ok(())
        })
        // Closing the window keeps the app alive in the menu bar (quit from the tray menu or Cmd+Q).
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::CloseRequested { api, .. } if window.label() == "main" => {
                api.prevent_close();
                hide_main(window.app_handle());
            }
            // Popover behaves like a menu: it disappears as soon as it loses focus.
            tauri::WindowEvent::Focused(false) if window.label() == "popover" => {
                if !POPOVER_PINNED.load(Ordering::Relaxed) {
                    hide_popover(window.app_handle());
                }
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_dashboard_metrics,
            commands::get_previous_period,
            commands::test_connection,
            commands::discover_metrics,
            commands::get_prometheus_health,
            insights::get_insights_data,
            insights::get_local_stats_cache,
            sessions::get_sessions_data,
            tray::set_tray_alert,
            telemetry::get_telemetry_status,
            telemetry::enable_claude_telemetry,
            stack::ensure_monitoring_stack,
            stack::stop_monitoring_stack,
            stack::restart_monitoring_stack,
            stack::get_stack_logs,
            stack::get_setup_status,
            backup::backup_prometheus,
            backup::list_backups,
            backup::restore_prometheus,
            export::save_export,
            anomaly::get_cost_spike,
            usage::get_usage_windows,
            open_dashboard,
            quit_app,
            set_popover_pinned,
            get_popover_pinned,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            // Re-opening the app from Finder/Spotlight while it only lives in the menu bar brings the window back.
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Reopen { .. } = event {
                show_main(app);
            }
            #[cfg(not(target_os = "macos"))]
            let _ = (app, event);
        });
}
