//! Integration: sampling admits and drops spans before the durable write.
//!
//! Proves contract #4 (`rubix/STACK-DEISGN.md`): traces are sampled. A drop
//! fraction of `0.0` persists every span; `1.0` persists none; a middle fraction
//! thins the population toward its configured rate over a large sample. Verified
//! against the live `trace` table on kv-mem.

#[path = "open.rs"]
mod open;

use rubix_core::CorrelationId;
use rubix_trace::{Persisted, SampleRate, Span, assemble_trace, persist_span};

use open::{NS, open_trace_store};

fn span(trace: &CorrelationId, name: &str) -> Span {
    Span::root(trace.clone(), name, serde_json::json!({}), 0, 1)
}

#[tokio::test]
async fn drop_fraction_zero_persists_every_span() {
    let handle = open_trace_store("sample_keep_all").await;
    let trace = CorrelationId::carry("corr-keep");
    let rate = SampleRate::new(0.0);

    for i in 0..20 {
        let outcome = persist_span(handle.raw(), NS, &span(&trace, &format!("s{i}")), rate)
            .await
            .expect("persist span");
        assert_eq!(outcome, Persisted::Written);
    }

    let forest = assemble_trace(handle.raw(), &trace).await.expect("assemble");
    assert_eq!(forest.len(), 20, "every span persisted and read back");
}

#[tokio::test]
async fn drop_fraction_one_persists_no_span() {
    let handle = open_trace_store("sample_drop_all").await;
    let trace = CorrelationId::carry("corr-drop");
    let rate = SampleRate::new(1.0);

    for i in 0..20 {
        let outcome = persist_span(handle.raw(), NS, &span(&trace, &format!("s{i}")), rate)
            .await
            .expect("persist span");
        assert_eq!(outcome, Persisted::Dropped);
    }

    let forest = assemble_trace(handle.raw(), &trace).await.expect("assemble");
    assert!(forest.is_empty(), "no span persisted");
}

#[tokio::test]
async fn a_middle_fraction_thins_toward_its_configured_rate() {
    let handle = open_trace_store("sample_thin").await;
    let trace = CorrelationId::carry("corr-thin");
    let rate = SampleRate::new(0.5);

    let total = 400;
    let mut written = 0;
    for i in 0..total {
        let outcome = persist_span(handle.raw(), NS, &span(&trace, &format!("s{i}")), rate)
            .await
            .expect("persist span");
        if outcome == Persisted::Written {
            written += 1;
        }
    }

    let kept = f64::from(written) / f64::from(total);
    assert!((kept - 0.5).abs() < 0.1, "kept fraction {kept} near 0.5");

    let forest = assemble_trace(handle.raw(), &trace).await.expect("assemble");
    assert_eq!(forest.len(), written as usize, "stored count matches writes");
}
