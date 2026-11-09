//! System tray title: connection dot + 24h cost (in the user's currency) + budget marker.
//!
//! The title is written only by a Rust background poller, so it stays live while every window is hidden
//! (a hidden webview is not reliable for timers).

use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

use tauri::{tray::TrayIcon, AppHandle, Manager, State};

use crate::prometheus::PrometheusClient;

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

/// The few settings the poller needs, read from the plugin-store file (`settings.json` in the app data dir).
struct TraySettings {
    prometheus_url: String,
    currency: String,
    usd_to_idr: f64,
}

impl Default for TraySettings {
    fn default() -> Self {
        Self { prometheus_url: "http://localhost:9090".into(), currency: "IDR".into(), usd_to_idr: 16000.0 }
    }
}

fn settings_path(app: &AppHandle) -> Option<PathBuf> {
    app.path().app_data_dir().ok().map(|d| d.join("settings.json"))
}

fn read_settings(app: &AppHandle) -> TraySettings {
    let mut out = TraySettings::default();
    let Some(path) = settings_path(app) else { return out };
    let Ok(text) = std::fs::read_to_string(path) else { return out };
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) else { return out };
    let s = &json["settings"];
    if let Some(u) = s["prometheusUrl"].as_str().filter(|u| !u.is_empty()) { out.prometheus_url = u.to_string(); }
    if let Some(c) = s["currency"].as_str() { out.currency = c.to_string(); }
    if let Some(r) = s["usdToIdrRate"].as_f64().filter(|r| *r > 0.0) { out.usd_to_idr = r; }
    out
}

/// Compact cost for the menu bar: "Rp 165rb" / "Rp 1,25jt" for IDR, "$10.32" for USD.
fn format_cost_short(cost_usd: f64, currency: &str, usd_to_idr: f64) -> String {
    if currency == "IDR" {
        let idr = cost_usd * usd_to_idr;
        if idr >= 1_000_000.0 {
            format!("Rp {:.2}jt", idr / 1_000_000.0).replace('.', ",")
        } else if idr >= 1_000.0 {
            format!("Rp {:.0}rb", idr / 1_000.0)
        } else {
            format!("Rp {:.0}", idr)
        }
    } else if cost_usd >= 1.0 {
        format!("${:.2}", cost_usd)
    } else {
        format!("${:.3}", cost_usd)
    }
}

fn render_title(connected: bool, cost_usd: Option<f64>, alert: u8, currency: &str, usd_to_idr: f64) -> String {
    let dot = if connected { "🟢" } else { "🔴" };
    let marker = match alert {
        2 => " 🚨",
        1 => " ⚠️",
        _ => "",
    };
    match cost_usd {
        Some(c) => format!("{} {}{}", dot, format_cost_short(c, currency, usd_to_idr), marker),
        None => format!("{} --{}", dot, marker),
    }
}

/// Query the last-24h cost and refresh the tray title every 30 seconds, forever.
pub fn start_poller(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut last_cost: Option<f64> = None;
        loop {
            let cfg = read_settings(&app);
            let client = PrometheusClient::new(&cfg.prometheus_url);
            let connected = match client.query("sum(increase(claude_code_cost_usage_USD_total[1d]))").await {
                Ok(rows) => {
                    last_cost = Some(
                        rows.first()
                            .and_then(|r| r.value.as_ref())
                            .and_then(|(_, v)| v.parse::<f64>().ok())
                            .unwrap_or(0.0),
                    );
                    true
                }
                Err(_) => false, // keep showing the last known cost, with the red dot
            };

            let state: State<TrayState> = app.state();
            let alert = state.alert.lock().map(|a| *a).unwrap_or(0);
            let title = render_title(connected, last_cost, alert, &cfg.currency, cfg.usd_to_idr);
            if let Ok(guard) = state.tray.lock() {
                if let Some(tray) = guard.as_ref() {
                    let _ = tray.set_title(Some(&title));
                }
            }

            tokio::time::sleep(Duration::from_secs(30)).await;
        }
    });
}

/// Set the budget alert level shown next to the tray cost (0 ok, 1 warn, 2 over).
#[tauri::command]
pub fn set_tray_alert(tray_state: State<'_, TrayState>, level: u8) -> Result<(), String> {
    *tray_state.alert.lock().map_err(|e| e.to_string())? = level.min(2);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idr_is_compact() {
        assert_eq!(format_cost_short(0.01, "IDR", 16000.0), "Rp 160");
        assert_eq!(format_cost_short(10.32, "IDR", 16000.0), "Rp 165rb");
        assert_eq!(format_cost_short(100.0, "IDR", 16000.0), "Rp 1,60jt");
    }

    #[test]
    fn usd_keeps_cents() {
        assert_eq!(format_cost_short(10.321, "USD", 16000.0), "$10.32");
        assert_eq!(format_cost_short(0.1234, "USD", 16000.0), "$0.123");
    }

    #[test]
    fn title_shows_connection_alert_and_unknown_cost() {
        assert_eq!(render_title(true, Some(10.0), 0, "USD", 1.0), "🟢 $10.00");
        assert_eq!(render_title(false, Some(10.0), 2, "USD", 1.0), "🔴 $10.00 🚨");
        assert_eq!(render_title(false, None, 1, "USD", 1.0), "🔴 -- ⚠️");
    }
}
