//! Claude Code usage windows (5-hour blocks + rolling 7 days), computed from the
//! local transcripts in `~/.claude/projects/*/*.jsonl`. No Prometheus needed.
//!
//! Counted tokens = input + output + cache_creation (cache reads are excluded, as they
//! are not what plan limits are usually measured on).
//!
//! TODO(usage-window-cost): Add `blockCostUsd` once model pricing lives in Rust (currently only in the
//!   frontend/Prometheus path). Expected: same cost as the Sessions view for the same time span.

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::time::{Duration, SystemTime};

use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;

const BLOCK_HOURS: i64 = 5;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageWindows {
    /// Whether a block is currently running (now < blockEnd).
    pub block_active: bool,
    /// Unix seconds; 0 when there is no active block.
    pub block_start: i64,
    pub block_end: i64,
    pub block_tokens: u64,
    pub minutes_remaining: i64,
    pub week_tokens: u64,
}

fn floor_to_hour(ts: i64) -> i64 {
    ts - ts.rem_euclid(3600)
}

fn collect_events(since: DateTime<Utc>) -> Vec<(i64, u64)> {
    let Some(home) = dirs::home_dir() else { return vec![] };
    let root = home.join(".claude").join("projects");
    let Ok(projects) = fs::read_dir(root) else { return vec![] };

    // Skip files not touched since `since` (cheap pre-filter; transcripts are append-only).
    let min_mtime = SystemTime::from(since) - Duration::from_secs(60);

    // message.id -> (timestamp, tokens); streaming writes the same message several times.
    let mut by_id: HashMap<String, (i64, u64)> = HashMap::new();

    for proj in projects.flatten() {
        let Ok(files) = fs::read_dir(proj.path()) else { continue };
        for f in files.flatten() {
            let path = f.path();
            if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                continue;
            }
            if f.metadata().and_then(|m| m.modified()).map(|m| m < min_mtime).unwrap_or(true) {
                continue;
            }
            let Ok(file) = File::open(&path) else { continue };
            for line in BufReader::new(file).lines().map_while(Result::ok) {
                if !line.contains("\"usage\"") {
                    continue;
                }
                let Ok(v) = serde_json::from_str::<Value>(&line) else { continue };
                let Some(ts) = v.get("timestamp").and_then(|t| t.as_str()) else { continue };
                let Ok(ts) = DateTime::parse_from_rfc3339(ts) else { continue };
                let ts = ts.timestamp();
                if ts < since.timestamp() {
                    continue;
                }
                let msg = v.get("message");
                let Some(u) = msg.and_then(|m| m.get("usage")) else { continue };
                let n = |k: &str| u.get(k).and_then(|x| x.as_u64()).unwrap_or(0);
                let tokens = n("input_tokens") + n("output_tokens") + n("cache_creation_input_tokens");
                let id = msg
                    .and_then(|m| m.get("id"))
                    .and_then(|i| i.as_str())
                    .map(String::from)
                    .unwrap_or_else(|| format!("{}-{}", path.display(), ts));
                // Keep the largest count seen for a message (last streamed chunk has final totals).
                let e = by_id.entry(id).or_insert((ts, tokens));
                if tokens >= e.1 {
                    *e = (ts, tokens);
                }
            }
        }
    }

    let mut events: Vec<(i64, u64)> = by_id.into_values().collect();
    events.sort_by_key(|e| e.0);
    events
}

fn compute(events: &[(i64, u64)], now: i64) -> UsageWindows {
    let week_start = now - 7 * 24 * 3600;
    let week_tokens: u64 = events.iter().filter(|e| e.0 >= week_start).map(|e| e.1).sum();

    // Walk events building 5h blocks; remember the last one.
    let mut block: Option<(i64, i64, u64)> = None; // (start, end, tokens)
    for &(ts, tokens) in events {
        match block {
            Some((s, e, t)) if ts < e => block = Some((s, e, t + tokens)),
            _ => {
                let start = floor_to_hour(ts);
                block = Some((start, start + BLOCK_HOURS * 3600, tokens));
            }
        }
    }

    match block {
        Some((s, e, t)) if now < e => UsageWindows {
            block_active: true,
            block_start: s,
            block_end: e,
            block_tokens: t,
            minutes_remaining: (e - now) / 60,
            week_tokens,
        },
        _ => UsageWindows {
            block_active: false,
            block_start: 0,
            block_end: 0,
            block_tokens: 0,
            minutes_remaining: 0,
            week_tokens,
        },
    }
}

#[tauri::command]
pub async fn get_usage_windows() -> Result<UsageWindows, String> {
    tokio::task::spawn_blocking(|| {
        let now = Utc::now();
        // 7 days of history covers both the weekly total and any block still running.
        let events = collect_events(now - chrono::Duration::days(7));
        compute(&events, now.timestamp())
    })
    .await
    .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn groups_events_into_hour_aligned_5h_blocks() {
        let t0 = 1_700_000_000 - 1_700_000_000 % 3600 + 600; // 10 min past an hour
        let ev = vec![(t0, 100), (t0 + 3600, 50), (t0 + 6 * 3600, 7)];
        // now is 1h after the third event: second block (start floor(t0+6h)) is active
        let w = compute(&ev, t0 + 7 * 3600);
        assert!(w.block_active);
        assert_eq!(w.block_tokens, 7);
        assert_eq!(w.block_start, floor_to_hour(t0 + 6 * 3600));
        // block ends at H+11h, now = H+7h+10min => 3h50m = 230 min left
        assert_eq!(w.minutes_remaining, 230);
    }

    #[test]
    fn inactive_when_last_block_expired() {
        let t0 = 1_700_000_000 - 1_700_000_000 % 3600;
        let w = compute(&[(t0, 10)], t0 + 6 * 3600);
        assert!(!w.block_active);
        assert_eq!(w.week_tokens, 10);
    }
}
