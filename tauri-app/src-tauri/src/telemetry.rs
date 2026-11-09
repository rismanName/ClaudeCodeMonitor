//! One-click setup of Claude Code telemetry.
//!
//! Writes the OTEL variables into the `env` block of `~/.claude/settings.json` (read by every
//! Claude Code launch: terminal, VS Code, desktop) instead of a shell rc file.
//! The write is idempotent and the previous file is kept as `settings.json.monitor-bak`.

use std::fs;
use std::path::PathBuf;

use serde::Serialize;
use serde_json::{json, Map, Value};

const WANTED: &[(&str, &str)] = &[
    ("CLAUDE_CODE_ENABLE_TELEMETRY", "1"),
    ("OTEL_METRICS_EXPORTER", "otlp"),
    ("OTEL_EXPORTER_OTLP_PROTOCOL", "grpc"),
    ("OTEL_EXPORTER_OTLP_ENDPOINT", "http://localhost:4317"),
];

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TelemetryStatus {
    /// All wanted variables are present with the wanted values.
    pub enabled: bool,
    pub settings_path: String,
}

fn settings_path() -> Result<PathBuf, String> {
    dirs::home_dir()
        .map(|h| h.join(".claude").join("settings.json"))
        .ok_or_else(|| "Could not find home directory".to_string())
}

fn read_settings(path: &PathBuf) -> Result<Value, String> {
    match fs::read_to_string(path) {
        Ok(s) if s.trim().is_empty() => Ok(json!({})),
        Ok(s) => serde_json::from_str(&s).map_err(|e| format!("settings.json is not valid JSON: {e}")),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(json!({})),
        Err(e) => Err(e.to_string()),
    }
}

fn is_enabled(settings: &Value) -> bool {
    let env = settings.get("env").and_then(|e| e.as_object());
    WANTED
        .iter()
        .all(|(k, v)| env.and_then(|e| e.get(*k)).and_then(|x| x.as_str()) == Some(*v))
}

#[tauri::command]
pub fn get_telemetry_status() -> Result<TelemetryStatus, String> {
    let path = settings_path()?;
    let settings = read_settings(&path)?;
    Ok(TelemetryStatus {
        enabled: is_enabled(&settings),
        settings_path: path.display().to_string(),
    })
}

#[tauri::command]
pub fn enable_claude_telemetry() -> Result<TelemetryStatus, String> {
    let path = settings_path()?;
    let mut settings = read_settings(&path)?;
    if !settings.is_object() {
        return Err("settings.json root is not an object".into());
    }

    if !is_enabled(&settings) {
        if path.exists() {
            let backup = path.with_file_name("settings.json.monitor-bak");
            fs::copy(&path, &backup).map_err(|e| format!("backup failed: {e}"))?;
        }
        let root = settings.as_object_mut().unwrap();
        let env = root.entry("env").or_insert_with(|| Value::Object(Map::new()));
        let env = env
            .as_object_mut()
            .ok_or_else(|| "settings.json \"env\" is not an object".to_string())?;
        for (k, v) in WANTED {
            env.insert((*k).to_string(), Value::String((*v).to_string()));
        }
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let mut out = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
        out.push('\n');
        fs::write(&path, out).map_err(|e| e.to_string())?;
    }

    Ok(TelemetryStatus {
        enabled: true,
        settings_path: path.display().to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_enabled_only_when_all_vars_match() {
        let mut s = json!({ "env": {} });
        assert!(!is_enabled(&s));
        for (k, v) in WANTED {
            s["env"][*k] = json!(*v);
        }
        assert!(is_enabled(&s));
        s["env"]["OTEL_METRICS_EXPORTER"] = json!("console");
        assert!(!is_enabled(&s));
    }
}
