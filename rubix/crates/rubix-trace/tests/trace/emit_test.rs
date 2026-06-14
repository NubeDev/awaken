//! Integration: emitting a multi-span operation onto the WS-07 control bus.
//!
//! Proves the bus side of tracing (`rubix/docs/SCOPE.md`, "Tracing"): subscribe
//! to the span event type, emit a root span and a child span threaded by the same
//! WS-05 correlation id, and assert each arrives carrying its span detail and the
//! trace id as the event correlation id (contract #3, `rubix/STACK-DEISGN.md`).

use std::time::Duration;

use rubix_bus::{ControlBus, subscribe};
use rubix_core::CorrelationId;
use rubix_trace::{SPAN_EVENT_TYPE, Span, emit_span};

/// A broadcast delivery is asynchronous; bound the wait so a missing event fails
/// the test rather than hanging it.
const RECV_TIMEOUT: Duration = Duration::from_secs(5);

#[tokio::test]
async fn emitting_spans_delivers_them_to_a_span_subscriber() {
    let bus = ControlBus::new();
    let mut sub = subscribe(&bus, SPAN_EVENT_TYPE);

    let trace = CorrelationId::carry("corr-emit");
    let root = Span::root(trace.clone(), "ingest", serde_json::json!({ "rows": 3 }), 0, 100);
    let child = Span::child(&root, "rule.eval", serde_json::json!({ "fired": true }), 10, 90);

    assert_eq!(emit_span(&bus, &root), 1, "one subscriber reached");
    assert_eq!(emit_span(&bus, &child), 1, "one subscriber reached");

    let first = recv(&mut sub).await;
    assert_eq!(first.correlation_id(), &trace);
    assert_eq!(first.payload()["name"], "ingest");
    assert_eq!(first.payload()["parent_span_id"], serde_json::Value::Null);

    let second = recv(&mut sub).await;
    assert_eq!(second.correlation_id(), &trace);
    assert_eq!(second.payload()["name"], "rule.eval");
    assert_eq!(second.payload()["parent_span_id"], root.span_id.as_str());
}

#[tokio::test]
async fn emitting_with_no_subscriber_is_a_no_op() {
    let bus = ControlBus::new();
    let trace = CorrelationId::carry("corr-quiet");
    let span = Span::root(trace, "sink", serde_json::json!({}), 0, 1);
    assert_eq!(emit_span(&bus, &span), 0, "no subscribers, no failure");
}

async fn recv(sub: &mut rubix_bus::ControlSubscription) -> rubix_bus::ControlEvent {
    tokio::time::timeout(RECV_TIMEOUT, sub.recv())
        .await
        .expect("an event within the timeout")
        .expect("the channel stayed open")
}
