//! System tray title: connection dot + cost + budget marker.

use std::sync::Mutex;
use tauri::{tray::TrayIcon, State};

/// State to hold the tray icon handle and the current budget alert level
pub struct TrayState {
    pub tray: Mutex<Option<TrayIcon>>,
    /// 0 = ok, 1 = >=80% of budget, 2 = >=100% of budget
    pub alert: Mutex<u8>,
}

impl TrayState {
    pub fn new() -> Self {
        Self {
            tray: Mutex::new(None),
            alert: Mutex::new(0),
        }
    }
}

/// Format cost for tray display
fn format_cost_short(cost: f64) -> String {
    if cost >= 1.0 {
        format!("${:.2}", cost)
    } else {
        format!("${:.3}", cost)
    }
}

/// Update the system tray with current stats
#[tauri::command]
pub async fn update_tray_stats(
    tray_state: State<'_, TrayState>,
    total_cost: f64,
    is_connected: bool,
) -> Result<(), String> {
    let tray_guard = tray_state.tray.lock().map_err(|e| e.to_string())?;
    if let Some(tray) = tray_guard.as_ref() {
        // Green circle for connected, red for disconnected
        let status_indicator = if is_connected { "🟢" } else { "🔴" };
        let marker = match tray_state.alert.lock().map(|a| *a).unwrap_or(0) {
            2 => " 🚨",
            1 => " ⚠️",
            _ => "",
        };
        let title = format!("{} {}{}", status_indicator, format_cost_short(total_cost), marker);
        tray.set_title(Some(&title))
            .map_err(|e| format!("Failed to set tray title: {}", e))?;
    }
    Ok(())
}

/// Set the budget alert level shown next to the tray cost (0 ok, 1 warn, 2 over).
#[tauri::command]
pub fn set_tray_alert(tray_state: State<'_, TrayState>, level: u8) -> Result<(), String> {
    *tray_state.alert.lock().map_err(|e| e.to_string())? = level.min(2);
    Ok(())
}
