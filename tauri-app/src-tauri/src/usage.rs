//! Claude Code usage windows (5-hour sessions + weekly period), computed from the
//! local transcripts in `~/.claude/projects/*/*.jsonl`. No Prometheus needed.
//!
//! "Tokens" for plan limits = input + output + cache_creation (cache reads are excluded, as they
//! are not what plan limits are usually measured on). Cost uses every token type via `pricing`.

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::time::{Duration, SystemTime};

use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;

use crate::pricing::{self, Tokens};

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
    pub block_cost_usd: f64,
    pub minutes_remaining: i64,
    pub week_tokens: u64,
    pub week_cost_usd: f64,
    /// Estimated savings from cache reads over the last 7 days.
    pub week_cache_saved_usd: f64,
}

/// One assistant message with usage.
#[derive(Debug, Clone)]
struct Event {
    ts: i64,
    model: String,
    tokens: Tokens,
}

impl Event {
    fn limit_tokens(&self) -> u64 {
        self.tokens.input + self.tokens.output + self.tokens.cache_create
    }
    fn cost(&self, provider: &str) -> f64 {
        pricing::cost_usd(&pricing::rates(&self.model, provider), &self.tokens)
    }
}

fn collect_events(since: DateTime<Utc>) -> Vec<Event> {
    let Some(home) = dirs::home_dir() else { return vec![] };
    let root = home.join(".claude").join("projects");
    let Ok(projects) = fs::read_dir(root) else { return vec![] };

    // Skip files not touched since `since` (cheap pre-filter; transcripts are append-only).
    let min_mtime = SystemTime::from(since) - Duration::from_secs(60);

    // message.id -> event; streaming writes the same message several times.
    let mut by_id: HashMap<String, Event> = HashMap::new();

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
                let ev = Event {
                    ts,
                    model: msg
                        .and_then(|m| m.get("model"))
                        .and_then(|m| m.as_str())
                        .unwrap_or("")
                        .to_string(),
                    tokens: Tokens {
                        input: n("input_tokens"),
                        output: n("output_tokens"),
                        cache_read: n("cache_read_input_tokens"),
                        cache_create: n("cache_creation_input_tokens"),
                    },
                };
                let id = msg
                    .and_then(|m| m.get("id"))
                    .and_then(|i| i.as_str())
                    .map(String::from)
                    .unwrap_or_else(|| format!("{}-{}", path.display(), ts));
                // Keep the record with the most output seen for a message (last streamed chunk has final totals).
                match by_id.get(&id) {
                    Some(prev) if prev.tokens.output > ev.tokens.output => {}
                    _ => {
                        by_id.insert(id, ev);
                    }
                }
            }
        }
    }

    let mut events: Vec<Event> = by_id.into_values().collect();
    events.sort_by_key(|e| e.ts);
    events
}

/// `week_start` = start of the current weekly period (unix seconds, from the user's weekly reset day/time);
/// `None` falls back to a rolling 7 days. It is clamped so a bad value can never reach into the future or
/// further back than the 7+ days of history that were loaded.
fn compute(events: &[Event], now: i64, provider: &str, week_start: Option<i64>) -> UsageWindows {
    let week_start = week_start.map_or(now - 7 * 24 * 3600, |w| w.clamp(now - 8 * 24 * 3600, now));
    let week: Vec<&Event> = events.iter().filter(|e| e.ts >= week_start).collect();
    let week_tokens: u64 = week.iter().map(|e| e.limit_tokens()).sum();
    let week_cost_usd: f64 = week.iter().map(|e| e.cost(provider)).sum();
    let week_cache_saved_usd: f64 = week
        .iter()
        .map(|e| pricing::cache_saved_usd(&pricing::rates(&e.model, provider), e.tokens.cache_read))
        .sum();

    // Walk events building 5h blocks; remember the last one: (start, end, tokens, cost).
    let mut block: Option<(i64, i64, u64, f64)> = None;
    for e in events {
        match block {
            Some((s, end, t, c)) if e.ts < end => block = Some((s, end, t + e.limit_tokens(), c + e.cost(provider))),
            _ => {
                // A session starts exactly at its first message (Claude Desktop shows resets like 7:20 PM, not 7:00 PM).
                let start = e.ts;
                block = Some((start, start + BLOCK_HOURS * 3600, e.limit_tokens(), e.cost(provider)));
            }
        }
    }

    match block {
        Some((s, e, t, c)) if now < e => UsageWindows {
            block_active: true,
            block_start: s,
            block_end: e,
            block_tokens: t,
            block_cost_usd: c,
            minutes_remaining: (e - now) / 60,
            week_tokens,
            week_cost_usd,
            week_cache_saved_usd,
        },
        _ => UsageWindows {
            block_active: false,
            block_start: 0,
            block_end: 0,
            block_tokens: 0,
            block_cost_usd: 0.0,
            minutes_remaining: 0,
            week_tokens,
            week_cost_usd,
            week_cache_saved_usd,
        },
    }
}

#[tauri::command]
pub async fn get_usage_windows(pricing_provider: Option<String>, week_start: Option<i64>) -> Result<UsageWindows, String> {
    let provider = pricing_provider.unwrap_or_else(|| "anthropic".into());
    tokio::task::spawn_blocking(move || {
        let now = Utc::now();
        // 8 days of history covers a full weekly period (even one that just started a week ago) and any block still running.
        let events = collect_events(now - chrono::Duration::days(8));
        compute(&events, now.timestamp(), &provider, week_start)
    })
    .await
    .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(ts: i64, input: u64, output: u64) -> Event {
        Event { ts, model: "claude-sonnet-4".into(), tokens: Tokens { input, output, ..Default::default() } }
    }

    #[test]
    fn sessions_start_at_their_first_message_and_last_5h() {
        let t0 = 1_700_000_000 - 1_700_000_000 % 3600 + 1200; // 20 min past an hour
        let evs = vec![ev(t0, 100, 0), ev(t0 + 3600, 50, 0), ev(t0 + 6 * 3600, 7, 0)];
        // The third event is past the first session's end (t0 + 5h), so it opens a new one at its exact time.
        let w = compute(&evs, t0 + 7 * 3600, "anthropic", None);
        assert!(w.block_active);
        assert_eq!(w.block_tokens, 7);
        assert_eq!(w.block_start, t0 + 6 * 3600);
        assert_eq!(w.block_end, t0 + 11 * 3600); // resets at :20, like Claude Desktop
        assert_eq!(w.minutes_remaining, 240);
    }

    #[test]
    fn messages_inside_the_5h_window_join_the_same_session() {
        let t0 = 1_700_000_000 - 1_700_000_000 % 3600 + 1200;
        let evs = vec![ev(t0, 100, 0), ev(t0 + 4 * 3600, 50, 0)];
        let w = compute(&evs, t0 + 4 * 3600 + 60, "anthropic", None);
        assert_eq!(w.block_tokens, 150);
        assert_eq!(w.block_start, t0);
    }

    #[test]
    fn inactive_when_last_block_expired() {
        let t0 = 1_700_000_000 - 1_700_000_000 % 3600;
        let w = compute(&[ev(t0, 10, 0)], t0 + 6 * 3600, "anthropic", None);
        assert!(!w.block_active);
        assert_eq!(w.week_tokens, 10);
    }

    #[test]
    fn block_cost_uses_model_rates() {
        let t0 = 1_700_000_000 - 1_700_000_000 % 3600;
        // 1M input @ $3 + 1M output @ $15 = $18 (sonnet)
        let w = compute(&[ev(t0, 1_000_000, 1_000_000)], t0 + 60, "anthropic", None);
        assert!((w.block_cost_usd - 18.0).abs() < 1e-9);
        assert!((w.week_cost_usd - 18.0).abs() < 1e-9);
    }

    #[test]
    fn week_counts_only_since_the_weekly_reset() {
        let now = 1_700_000_000 - 1_700_000_000 % 3600 + 7 * 24 * 3600;
        let before_reset = ev(now - 4 * 24 * 3600, 1000, 0);
        let after_reset = ev(now - 2 * 24 * 3600, 30, 0);
        let evs = [before_reset, after_reset];
        // Rolling 7 days sees both; a reset 3 days ago excludes the first.
        assert_eq!(compute(&evs, now, "anthropic", None).week_tokens, 1030);
        assert_eq!(compute(&evs, now, "anthropic", Some(now - 3 * 24 * 3600)).week_tokens, 30);
    }

    #[test]
    fn bad_week_start_is_clamped() {
        let now = 1_700_000_000;
        let e = ev(now - 3600, 5, 0);
        // A reset "in the future" clamps to now => nothing counted yet.
        assert_eq!(compute(&[e.clone()], now, "anthropic", Some(now + 999_999)).week_tokens, 0);
        // A reset absurdly far back clamps to 8 days => the event is counted.
        assert_eq!(compute(&[e], now, "anthropic", Some(0)).week_tokens, 5);
    }
}
