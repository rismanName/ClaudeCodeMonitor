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

use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, State,
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
                .show_menu_on_left_click(true) // like Docker Desktop: click opens the menu
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
                        ..
                    } = event
                    {
                        show_main(tray.app_handle());
                    }
                })
                .build(app)?;

            // Store tray handle in state for later updates
            let tray_state: State<TrayState> = app.state();
            if let Ok(mut guard) = tray_state.tray.lock() {
                *guard = Some(tray);
            }

            Ok(())
        })
        // Closing the window keeps the app alive in the menu bar (quit from the tray menu or Cmd+Q).
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
                    api.prevent_close();
                    hide_main(window.app_handle());
                }
            }
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
            tray::update_tray_stats,
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
