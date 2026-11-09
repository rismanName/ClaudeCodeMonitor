//! Per-model token prices (USD per million tokens), shared by the local-data views.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rates {
    pub input: f64,
    pub output: f64,
    pub cache_read: f64,
    pub cache_create: f64,
}

/// Rates for a model id (e.g. "claude-sonnet-5-5") under a pricing provider.
/// cache_read = 10% of input, cache_create = 125% of input; Vertex carries a 10% premium.
/// Unknown models fall back to Sonnet pricing.
pub fn rates(model: &str, provider: &str) -> Rates {
    let m = model.to_lowercase();
    let (input, output) = if m.contains("opus") {
        (5.0, 25.0)
    } else if m.contains("haiku") {
        (1.0, 5.0)
    } else {
        (3.0, 15.0)
    };
    let k = if provider == "google-vertex" { 1.1 } else { 1.0 };
    Rates {
        input: input * k,
        output: output * k,
        cache_read: input * 0.10 * k,
        cache_create: input * 1.25 * k,
    }
}

/// Token counts of one usage record.
#[derive(Debug, Clone, Copy, Default)]
pub struct Tokens {
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub cache_create: u64,
}

pub fn cost_usd(r: &Rates, t: &Tokens) -> f64 {
    let per_m = |n: u64, rate: f64| n as f64 / 1_000_000.0 * rate;
    per_m(t.input, r.input) + per_m(t.output, r.output) + per_m(t.cache_read, r.cache_read) + per_m(t.cache_create, r.cache_create)
}

/// What cache reads saved versus paying the full input price for the same tokens.
pub fn cache_saved_usd(r: &Rates, cache_read_tokens: u64) -> f64 {
    cache_read_tokens as f64 / 1_000_000.0 * (r.input - r.cache_read)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_previous_inline_table() {
        // Values previously hard-coded in insights.rs
        assert_eq!(rates("claude-opus-4-5", "anthropic"), Rates { input: 5.0, output: 25.0, cache_read: 0.5, cache_create: 6.25 });
        assert_eq!(rates("claude-3-5-haiku", "aws-bedrock"), Rates { input: 1.0, output: 5.0, cache_read: 0.1, cache_create: 1.25 });
        let v = rates("claude-sonnet-4", "google-vertex");
        assert!((v.input - 3.3).abs() < 1e-9 && (v.output - 16.5).abs() < 1e-9);
        assert!((v.cache_read - 0.33).abs() < 1e-9 && (v.cache_create - 4.125).abs() < 1e-9);
    }

    #[test]
    fn cache_savings_is_input_minus_cache_read() {
        let r = rates("sonnet", "anthropic");
        assert!((cache_saved_usd(&r, 1_000_000) - 2.7).abs() < 1e-9);
    }
}
