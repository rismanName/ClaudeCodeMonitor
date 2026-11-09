//! Auto-start of the local monitoring stack (OTel Collector + Prometheus).
//!
//! Lets the desktop app be opened on its own: if Prometheus is not reachable,
//! the container engine (Docker Desktop) is launched and `compose up -d` is run.

// TODO(embedded-otlp): Replace the Docker dependency with an in-app OTLP receiver + local metrics store
//   (e.g. tonic OTLP gRPC on :4317 writing to SQLite). Expected: app works with zero containers;
//   Docker path stays as optional "advanced" mode. Pending decision: retention/storage format.
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use serde::Serialize;

/// Repo root at build time. Works for a locally built app; a relocated binary
/// must set `composeDir` in Settings.
const DEFAULT_COMPOSE_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");

#[cfg(target_os = "macos")]
const ENGINE_CANDIDATES: &[&str] = &[
    "/usr/local/bin/docker",
    "/opt/homebrew/bin/docker",
    "/Applications/Docker.app/Contents/Resources/bin/docker",
    "/usr/local/bin/podman",
    "/opt/homebrew/bin/podman",
];

#[cfg(target_os = "linux")]
const ENGINE_CANDIDATES: &[&str] = &[
    "/usr/bin/docker",
    "/usr/local/bin/docker",
    "/usr/bin/podman",
    "/usr/local/bin/podman",
];

#[cfg(target_os = "windows")]
const ENGINE_CANDIDATES: &[&str] = &[
    r"C:\Program Files\Docker\Docker\resources\bin\docker.exe",
    r"C:\Program Files\RedHat\Podman\podman.exe",
];

/// Best-effort start of the container engine app/service. Returns immediately; the caller polls `engine_ready`.
/// NOTE: the Linux and Windows branches are written from vendor docs and have NOT been run on those platforms.
fn launch_engine() {
    #[cfg(target_os = "macos")]
    {
        let _ = Command::new("open").args(["-ga", "Docker"]).status();
    }
    #[cfg(target_os = "linux")]
    {
        // Docker Desktop for Linux runs as a user service; plain Docker Engine needs root, so it is not started here.
        let _ = Command::new("systemctl").args(["--user", "start", "docker-desktop"]).status();
    }
    #[cfg(target_os = "windows")]
    {
        let _ = Command::new(r"C:\Program Files\Docker\Docker\Docker Desktop.exe").spawn();
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StackStatus {
    /// "running" | "started" | "error"
    pub state: String,
    /// Machine-readable code for the UI to translate:
    /// "already_running" | "started" | "no_engine" | "engine_timeout" | "no_compose" | "compose_failed"
    pub code: String,
    pub detail: String,
}

fn status(state: &str, code: &str, detail: impl Into<String>) -> StackStatus {
    StackStatus {
        state: state.into(),
        code: code.into(),
        detail: detail.into(),
    }
}

fn port_open(port: u16) -> bool {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    TcpStream::connect_timeout(&addr, Duration::from_millis(500)).is_ok()
}

pub(crate) fn find_engine() -> Option<PathBuf> {
    ENGINE_CANDIDATES
        .iter()
        .map(PathBuf::from)
        .find(|p| p.exists())
}

pub(crate) fn engine_ready(engine: &Path) -> bool {
    Command::new(engine)
        .arg("info")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn ensure_blocking(compose_dir: Option<String>) -> StackStatus {
    // Prometheus (9090) and the OTLP gRPC receiver (4317) both up => nothing to do.
    if port_open(9090) && port_open(4317) {
        return status("running", "already_running", "");
    }

    let Some(engine) = find_engine() else {
        return status("error", "no_engine", "docker/podman not found");
    };

    if !engine_ready(&engine) {
        launch_engine();

        let deadline = Instant::now() + Duration::from_secs(90);
        while !engine_ready(&engine) {
            if Instant::now() > deadline {
                return status("error", "engine_timeout", "container engine did not become ready in 90s");
            }
            std::thread::sleep(Duration::from_secs(2));
        }
    }

    let dir = compose_dir
        .filter(|d| !d.trim().is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_COMPOSE_DIR));
    let compose_file = dir.join("compose.yaml");
    if !compose_file.exists() {
        return status("error", "no_compose", compose_file.display().to_string());
    }

    match Command::new(&engine)
        .arg("compose")
        .arg("-f")
        .arg(&compose_file)
        .args(["up", "-d"])
        .output()
    {
        Ok(o) if o.status.success() => status("started", "started", ""),
        Ok(o) => status("error", "compose_failed", String::from_utf8_lossy(&o.stderr).trim().to_string()),
        Err(e) => status("error", "compose_failed", e.to_string()),
    }
}

/// Ensure Prometheus + OTel collector are running; start them if not.
#[tauri::command]
pub async fn ensure_monitoring_stack(compose_dir: Option<String>) -> Result<StackStatus, String> {
    tokio::task::spawn_blocking(move || ensure_blocking(compose_dir))
        .await
        .map_err(|e| e.to_string())
}

/// Run `compose <args>` against the project's compose.yaml (stop / restart / etc).
fn compose_blocking(compose_dir: Option<String>, args: &[&str]) -> StackStatus {
    let Some(engine) = find_engine() else {
        return status("error", "no_engine", "docker/podman not found");
    };
    if !engine_ready(&engine) {
        return status("error", "engine_timeout", "container engine is not running");
    }
    let dir = compose_dir
        .filter(|d| !d.trim().is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_COMPOSE_DIR));
    let compose_file = dir.join("compose.yaml");
    if !compose_file.exists() {
        return status("error", "no_compose", compose_file.display().to_string());
    }
    match Command::new(&engine).arg("compose").arg("-f").arg(&compose_file).args(args).output() {
        Ok(o) if o.status.success() => status("started", "started", ""),
        Ok(o) => status("error", "compose_failed", String::from_utf8_lossy(&o.stderr).trim().to_string()),
        Err(e) => status("error", "compose_failed", e.to_string()),
    }
}

/// Stop the monitoring containers (data volume is kept).
#[tauri::command]
pub async fn stop_monitoring_stack(compose_dir: Option<String>) -> Result<StackStatus, String> {
    tokio::task::spawn_blocking(move || compose_blocking(compose_dir, &["stop"]))
        .await
        .map_err(|e| e.to_string())
}

/// Restart the monitoring containers (e.g. after editing prometheus.yml).
#[tauri::command]
pub async fn restart_monitoring_stack(compose_dir: Option<String>) -> Result<StackStatus, String> {
    tokio::task::spawn_blocking(move || compose_blocking(compose_dir, &["restart"]))
        .await
        .map_err(|e| e.to_string())
}

/// Last `tail` lines of the compose stack's logs (both services, no ANSI colors).
#[tauri::command]
pub async fn get_stack_logs(compose_dir: Option<String>, tail: Option<u32>) -> Result<String, String> {
    tokio::task::spawn_blocking(move || {
        let engine = find_engine().ok_or("docker/podman not found")?;
        let dir = compose_dir
            .filter(|d| !d.trim().is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(DEFAULT_COMPOSE_DIR));
        let compose_file = dir.join("compose.yaml");
        if !compose_file.exists() {
            return Err(format!("compose.yaml not found: {}", compose_file.display()));
        }
        let out = Command::new(&engine)
            .arg("compose")
            .arg("-f")
            .arg(&compose_file)
            .args(["logs", "--no-color", "--tail"])
            .arg(tail.unwrap_or(200).min(2000).to_string())
            .output()
            .map_err(|e| e.to_string())?;
        // compose writes logs to stdout; errors to stderr.
        let mut text = String::from_utf8_lossy(&out.stdout).to_string();
        if !out.status.success() {
            text.push_str(&String::from_utf8_lossy(&out.stderr));
        }
        Ok(text)
    })
    .await
    .map_err(|e| e.to_string())?
}
