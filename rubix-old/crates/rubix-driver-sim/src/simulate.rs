//! Publish simulated `cur` samples on the configured point, on a cadence, until
//! shutdown. Samples are enqueued into a bounded [`CurBuffer`] and drained to the
//! [`ScopedSession`] (which authorizes each publish against the granted
//! capabilities before it reaches the bus). The `cur` channel is latest-wins: if
//! publishing falls behind the sample cadence the oldest queued samples are
//! dropped and counted, never silently truncated (STACK-DEISGN.md
//! "Ack/backpressure", `docs/sessions/WS-10.md`).

use rubix_driver::CurBuffer;

use crate::config::SimConfig;
use crate::scoped::ScopedSession;

/// Run the publish loop until `shutdown` resolves. A sample is a sine-like
/// oscillation around the baseline, JSON-encoded as a bare number on
/// `{point}/cur` — the same payload shape the server publishes for `cur`.
pub async fn run(
    session: &ScopedSession,
    cfg: &SimConfig,
    shutdown: impl std::future::Future<Output = ()>,
) {
    let cur_key = format!("{}/cur", cfg.point);

    // Fail closed: refuse to start if the configured point is outside the
    // granted publish scope. The scoped session would deny each publish anyway;
    // surfacing it here turns a silent loop into a clear startup error.
    if let Err(e) = cfg.caps.authorize_publish(&cfg.name, &cur_key) {
        tracing::error!(error = %e, key = %cur_key, "sim point not within granted capabilities; not publishing");
        return;
    }

    let mut buffer: CurBuffer<f64> = CurBuffer::new(cfg.cur_capacity);
    let mut ticker = tokio::time::interval(cfg.period);
    tokio::pin!(shutdown);
    let mut step: u64 = 0;
    let mut last_logged_drops: u64 = 0;
    loop {
        tokio::select! {
            _ = ticker.tick() => {
                let value = sample(cfg.baseline, cfg.amplitude, step);
                step = step.wrapping_add(1);
                buffer.push(value);
                drain(session, &cur_key, &mut buffer).await;
                log_drops(&cur_key, buffer.dropped(), &mut last_logged_drops);
            }
            _ = &mut shutdown => {
                tracing::info!(driver = %cfg.name, dropped = buffer.dropped(), "sim shutting down");
                return;
            }
        }
    }
}

/// Publish every queued sample in arrival order. A publish failure re-counts the
/// sample as a loss only via the bus error; the buffer is already drained, so a
/// transient bus error does not wedge the loop (the next tick enqueues fresh).
async fn drain(session: &ScopedSession, cur_key: &str, buffer: &mut CurBuffer<f64>) {
    while let Some(value) = buffer.pop() {
        match serde_json::to_vec(&value) {
            Ok(payload) => match session.put(cur_key, payload).await {
                Ok(()) => tracing::debug!(key = %cur_key, value, "published cur"),
                Err(e) => tracing::warn!(error = %e, key = %cur_key, "publish cur"),
            },
            Err(e) => tracing::error!(error = %e, "encode sample"),
        }
    }
}

/// Emit a single warning whenever the dropped-sample count has grown since the
/// last tick, so overflow is observable without spamming a line per drop.
fn log_drops(cur_key: &str, dropped: u64, last_logged: &mut u64) {
    if dropped > *last_logged {
        tracing::warn!(
            key = %cur_key,
            dropped,
            newly_dropped = dropped - *last_logged,
            "cur buffer overflow: dropped oldest samples under pressure"
        );
        *last_logged = dropped;
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

    #[test]
    fn log_drops_fires_only_on_growth() {
        let mut last = 0;
        log_drops("k/cur", 0, &mut last);
        assert_eq!(last, 0);
        log_drops("k/cur", 3, &mut last);
        assert_eq!(last, 3);
        // No growth: last is unchanged.
        log_drops("k/cur", 3, &mut last);
        assert_eq!(last, 3);
    }
}
