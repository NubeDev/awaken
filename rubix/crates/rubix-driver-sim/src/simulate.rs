//! Publish simulated `cur` samples on the configured point, on a cadence, until
//! shutdown. Each publish is self-authorized against the granted capabilities —
//! the driver refuses to publish outside its scope even though the bus also
//! enforces it, so a misconfigured `point` fails loudly here rather than being
//! silently dropped.

use zenoh::Session;

use rubix_driver::Access;

use crate::config::SimConfig;

/// Run the publish loop until `shutdown` resolves. A sample is a sine-like
/// oscillation around the baseline, JSON-encoded as a bare number on
/// `{point}/cur` — the same payload shape the server publishes for `cur`.
pub async fn run(session: &Session, cfg: &SimConfig, shutdown: impl std::future::Future<Output = ()>) {
    let cur_key = format!("{}/cur", cfg.point);

    // Fail closed: refuse to start if the configured point is outside the
    // granted publish scope. The bus would drop it anyway; surfacing it here
    // turns a silent no-op into a clear startup error.
    if let Err(e) = cfg.caps.authorize_publish(&cfg.name, &cur_key) {
        tracing::error!(error = %e, key = %cur_key, "sim point not within granted capabilities; not publishing");
        return;
    }
    debug_assert!(cfg.caps.allows(&cur_key, Access::Publish));

    let mut ticker = tokio::time::interval(cfg.period);
    tokio::pin!(shutdown);
    let mut step: u64 = 0;
    loop {
        tokio::select! {
            _ = ticker.tick() => {
                let value = sample(cfg.baseline, cfg.amplitude, step);
                step = step.wrapping_add(1);
                match serde_json::to_vec(&value) {
                    Ok(payload) => {
                        if let Err(e) = session.put(&cur_key, payload).await {
                            tracing::warn!(error = %e, key = %cur_key, "publish cur");
                        } else {
                            tracing::debug!(key = %cur_key, value, "published cur");
                        }
                    }
                    Err(e) => tracing::error!(error = %e, "encode sample"),
                }
            }
            _ = &mut shutdown => {
                tracing::info!(driver = %cfg.name, "sim shutting down");
                return;
            }
        }
    }
}

/// A deterministic oscillation around `baseline` with peak deviation
/// `amplitude`, stepped each tick. Deterministic (no RNG) so a test asserting
/// the published value is stable.
fn sample(baseline: f64, amplitude: f64, step: u64) -> f64 {
    // 12-step cycle; cheap integer-driven triangle wave, rounded to 0.1.
    let phase = (step % 12) as f64 / 12.0; // 0.0..1.0
    let tri = 1.0 - (2.0 * phase - 1.0).abs(); // 0..1..0
    let raw = baseline + amplitude * (2.0 * tri - 1.0); // baseline ± amplitude
    (raw * 10.0).round() / 10.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_stays_within_baseline_plus_minus_amplitude() {
        for step in 0..48 {
            let v = sample(21.0, 2.0, step);
            assert!((19.0..=23.0).contains(&v), "step {step} -> {v}");
        }
    }

    #[test]
    fn sample_is_deterministic() {
        assert_eq!(sample(21.0, 2.0, 3), sample(21.0, 2.0, 15));
    }
}
