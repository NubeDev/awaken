//! Integration: the in-process control plane fans out by event type.
//!
//! Proves the in-process plane (`rubix/docs/SCOPE.md`, "Event bus"): a publish
//! reaches every subscriber of the event's type, and a subscription to one type
//! never receives another type's events.

use rubix_bus::{ControlBus, ControlEvent, publish, subscribe};
use rubix_core::CorrelationId;

#[tokio::test]
async fn publish_reaches_every_subscriber_of_the_type() {
    let bus = ControlBus::new();
    let mut first = subscribe(&bus, "record.created");
    let mut second = subscribe(&bus, "record.created");

    let event = ControlEvent::new(
        "record.created",
        serde_json::json!({ "id": "r1" }),
        CorrelationId::carry("corr-1"),
    );
    let reached = publish(&bus, event.clone());

    assert_eq!(reached, 2, "both subscribers are counted");
    assert_eq!(first.recv().await.expect("first receives"), event);
    assert_eq!(second.recv().await.expect("second receives"), event);
}

#[tokio::test]
async fn a_subscription_only_receives_its_own_event_type() {
    let bus = ControlBus::new();
    let mut created = subscribe(&bus, "record.created");
    let mut deleted = subscribe(&bus, "record.deleted");

    let created_event = ControlEvent::new(
        "record.created",
        serde_json::json!({ "id": "r1" }),
        CorrelationId::carry("corr-1"),
    );
    let reached = publish(&bus, created_event.clone());

    // The created event reaches only the created subscriber.
    assert_eq!(reached, 1);
    assert_eq!(created.recv().await.expect("created receives"), created_event);

    // The deleted subscriber sees nothing: publishing a deleted event and
    // reading it back proves the created event was never delivered to it.
    let deleted_event = ControlEvent::new(
        "record.deleted",
        serde_json::json!({ "id": "r1" }),
        CorrelationId::carry("corr-2"),
    );
    publish(&bus, deleted_event.clone());
    assert_eq!(deleted.recv().await.expect("deleted receives"), deleted_event);
}

#[tokio::test]
async fn publish_with_no_subscriber_is_a_noop() {
    let bus = ControlBus::new();
    let reached = publish(
        &bus,
        ControlEvent::new(
            "rule.fired",
            serde_json::json!({}),
            CorrelationId::carry("corr-1"),
        ),
    );
    assert_eq!(reached, 0, "no subscribers means a zero-reach no-op");
}
