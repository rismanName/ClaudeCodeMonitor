//! Prometheus data backup via the TSDB snapshot API.
//!
//! Requires Prometheus to run with `--web.enable-admin-api` (set in compose.yaml).
//! The snapshot is created inside the container, copied to ~/Downloads, then removed from the container.
//!
//! TODO(restore): Import a backup folder back into the prometheus_data volume.
//!   Expected: stop stack, copy snapshot blocks into /prometheus, start stack, verify `up` returns data; confirm dialog first
//!   because it overwrites current data.

use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::stack::find_engine;

const CONTAINER: &str = "prometheus";

#[derive(Deserialize)]
struct SnapshotResponse {
    status: String,
    data: Option<SnapshotData>,
    error: Option<String>,
}

#[derive(Deserialize)]
struct SnapshotData {
    name: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupResult {
    pub path: String,
}

/// A snapshot name is "<timestamp>-<hex>"; refuse anything else before it reaches a shell-like context.
fn valid_snapshot_name(name: &str) -> bool {
    !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == 'Z')
}

#[tauri::command]
pub async fn backup_prometheus(prometheus_url: String) -> Result<BackupResult, String> {
    let url = format!("{}/api/v1/admin/tsdb/snapshot", prometheus_url.trim_end_matches('/'));
    let resp = reqwest::Client::new()
        .post(&url)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if resp.status() == reqwest::StatusCode::METHOD_NOT_ALLOWED || resp.status() == reqwest::StatusCode::NOT_FOUND {
        return Err("admin_api_disabled".into());
    }
    let body: SnapshotResponse = resp.json().await.map_err(|e| e.to_string())?;
    if body.status != "success" {
        return Err(body.error.unwrap_or_else(|| "snapshot failed".into()));
    }
    let name = body.data.ok_or("snapshot response had no data")?.name;
    if !valid_snapshot_name(&name) {
        return Err(format!("unexpected snapshot name: {name}"));
    }

    tokio::task::spawn_blocking(move || {
        let engine = find_engine().ok_or("docker/podman not found")?;
        let dest_root = dirs::download_dir().ok_or("Could not find Downloads folder")?;
        let dest = dest_root.join(format!("prometheus-backup-{name}"));
        let cp = Command::new(&engine)
            .arg("cp")
            .arg(format!("{CONTAINER}:/prometheus/snapshots/{name}"))
            .arg(&dest)
            .output()
            .map_err(|e| e.to_string())?;
        if !cp.status.success() {
            return Err(String::from_utf8_lossy(&cp.stderr).trim().to_string());
        }
        // Best-effort cleanup of the in-container snapshot.
        let _ = Command::new(&engine)
            .args(["exec", CONTAINER, "rm", "-rf"])
            .arg(format!("/prometheus/snapshots/{name}"))
            .output();
        Ok(BackupResult { path: dest.display().to_string() })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::valid_snapshot_name;

    #[test]
    fn accepts_prometheus_names_and_rejects_paths() {
        assert!(valid_snapshot_name("20261109T031500Z-1a2b3c4d5e6f7a8b"));
        assert!(!valid_snapshot_name("../etc"));
        assert!(!valid_snapshot_name("a b"));
        assert!(!valid_snapshot_name(""));
    }
}
