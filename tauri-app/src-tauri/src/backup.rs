//! Prometheus data backup via the TSDB snapshot API.
//!
//! Requires Prometheus to run with `--web.enable-admin-api` (set in compose.yaml).
//! The snapshot is created inside the container, copied to ~/Downloads, then removed from the container.
//!
//! Restore is merge-only: blocks from a backup are copied into the data volume, nothing is deleted.

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

const BACKUP_PREFIX: &str = "prometheus-backup-";

/// Backup folders found in ~/Downloads, newest first (names only).
#[tauri::command]
pub fn list_backups() -> Vec<String> {
    let Some(dir) = dirs::download_dir() else { return vec![] };
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .map(|rd| {
            rd.flatten()
                .filter(|e| e.path().is_dir())
                .filter_map(|e| e.file_name().into_string().ok())
                .filter(|n| n.strip_prefix(BACKUP_PREFIX).map(valid_snapshot_name).unwrap_or(false))
                .collect()
        })
        .unwrap_or_default();
    names.sort_by(|a, b| b.cmp(a));
    names
}

/// A TSDB block directory is a 26-char ULID.
fn looks_like_block(name: &str) -> bool {
    name.len() == 26 && name.chars().all(|c| c.is_ascii_alphanumeric())
}

/// Merge a backup (from `list_backups`) into the running Prometheus data volume.
/// Sequence: stop container -> copy blocks in -> start -> fix ownership (docker cp writes root-owned files).
#[tauri::command]
pub async fn restore_prometheus(backup_name: String) -> Result<(), String> {
    if !backup_name.strip_prefix(BACKUP_PREFIX).map(valid_snapshot_name).unwrap_or(false) {
        return Err("invalid backup name".into());
    }
    tokio::task::spawn_blocking(move || {
        let engine = find_engine().ok_or("docker/podman not found")?;
        let src = dirs::download_dir().ok_or("Could not find Downloads folder")?.join(&backup_name);
        let has_blocks = std::fs::read_dir(&src)
            .map_err(|e| e.to_string())?
            .flatten()
            .any(|e| looks_like_block(&e.file_name().to_string_lossy()));
        if !has_blocks {
            return Err("folder contains no TSDB blocks".into());
        }

        let run = |args: &[&str]| -> Result<(), String> {
            let o = Command::new(&engine).args(args).output().map_err(|e| e.to_string())?;
            if o.status.success() { Ok(()) } else { Err(String::from_utf8_lossy(&o.stderr).trim().to_string()) }
        };

        run(&["stop", CONTAINER])?;
        // Always try to bring the container back, even if the copy failed.
        let copy = Command::new(&engine)
            .arg("cp")
            .arg(format!("{}/.", src.display()))
            .arg(format!("{CONTAINER}:/prometheus/"))
            .output()
            .map_err(|e| e.to_string())
            .and_then(|o| if o.status.success() { Ok(()) } else { Err(String::from_utf8_lossy(&o.stderr).trim().to_string()) });
        run(&["start", CONTAINER])?;
        copy?;
        run(&["exec", "-u", "root", CONTAINER, "chown", "-R", "65534:65534", "/prometheus"])
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::{looks_like_block, valid_snapshot_name};

    #[test]
    fn accepts_prometheus_names_and_rejects_paths() {
        assert!(valid_snapshot_name("20261109T031500Z-1a2b3c4d5e6f7a8b"));
        assert!(!valid_snapshot_name("../etc"));
        assert!(!valid_snapshot_name("a b"));
        assert!(!valid_snapshot_name(""));
    }

    #[test]
    fn recognizes_block_dirs() {
        assert!(looks_like_block("01M6HVDDKBQ63HMKVPPD5134Y5"));
        assert!(!looks_like_block("wal"));
        assert!(!looks_like_block("chunks_head"));
    }
}
