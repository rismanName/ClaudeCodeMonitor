//! Cost spike detection: last hour of spend vs the hourly average of the previous 24 hours.

use serde::Serialize;

use crate::prometheus::PrometheusClient;

/// Ignore spikes below this absolute spend (USD/hour) to avoid noise when the baseline is ~0.
const MIN_SPIKE_USD: f64 = 0.25;
const SPIKE_FACTOR: f64 = 3.0;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CostSpike {
    pub is_spike: bool,
    pub last_hour_usd: f64,
    pub avg_hourly_usd: f64,
}

pub fn is_spike(last_hour: f64, avg_hourly: f64) -> bool {
    last_hour >= MIN_SPIKE_USD && last_hour > SPIKE_FACTOR * avg_hourly
}

async fn scalar(client: &PrometheusClient, q: &str) -> Result<f64, String> {
    Ok(client
        .query(q)
        .await
        .map_err(|e| e.to_string())?
        .first()
        .and_then(|r| r.value.as_ref())
        .and_then(|(_, v)| v.parse::<f64>().ok())
        .unwrap_or(0.0))
}

#[tauri::command]
pub async fn get_cost_spike(prometheus_url: String) -> Result<CostSpike, String> {
    let client = PrometheusClient::new(&prometheus_url);
    let last_hour = scalar(&client, "sum(increase(claude_code_cost_usage_USD_total[1h]))").await?;
    // The 24h window ending one hour ago, averaged per hour.
    let prev_24h = scalar(&client, "sum(increase(claude_code_cost_usage_USD_total[24h] offset 1h))").await?;
    let avg = prev_24h / 24.0;
    Ok(CostSpike { is_spike: is_spike(last_hour, avg), last_hour_usd: last_hour, avg_hourly_usd: avg })
}

#[cfg(test)]
mod tests {
    use super::is_spike;

    #[test]
    fn flags_only_large_and_relative_jumps() {
        assert!(is_spike(1.0, 0.2)); // 5x and above floor
        assert!(!is_spike(0.1, 0.0)); // below absolute floor
        assert!(!is_spike(0.6, 0.3)); // only 2x
        assert!(is_spike(0.3, 0.0)); // new activity after quiet day
    }
}
