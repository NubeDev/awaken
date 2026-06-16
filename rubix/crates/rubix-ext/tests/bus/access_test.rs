//! Integration: an extension reaches the in-process control event bus only
//! through its WS-04 capability grants, checked fail closed against the same DB
//! grant table a user is held to.
//!
//! Four halves of one rule — the event bus is a gated plane, not an open handle:
//!
//! - **Subscribe is gated.** With the `event-subscribe` grant an extension opens
//!   a live subscription and observes an event published after it; without the
//!   grant the subscription is refused before any receiver opens (contract #2).
//! - **Publish is gated.** With the `event-publish` grant an extension fans an
//!   event out to a subscriber; without it the publish is refused before the
//!   event reaches anyone — observing the bus and emitting onto it are distinct
//!   authorities.

#[path = "../ext/mod.rs"]
mod ext;

use rubix_bus::{ControlBus, ControlEvent};
use rubix_core::CorrelationId;
use rubix_gate::{Capability, create_grant};

use ext::open::{admin, open_ext_store};

/// The tenant the extension is scoped to.
const TENANT: &str = "rubix";

#[tokio::test]
async fn a_granted_extension_subscribes_and_observes_an_event() {
    let handle = open_ext_store("ext_bus_subscribe").await;
    let bus = ControlBus::new();

    // A subscriber extension granted `event-subscribe`, and a publisher extension
    // granted `event-publish` — distinct principals, distinct authorities.
    let watcher = rubix_ext::register_extension(handle.raw(), "watcher-ext", TENANT, "k")
        .await
        .expect("register watcher")
        .principal()
        .clone();
    let emitter = rubix_ext::register_extension(handle.raw(), "emitter-ext", TENANT, "k")
        .await
        .expect("register emitter")
        .principal()
        .clone();
    create_grant(handle.raw(), &admin(), &watcher, Capability::EventSubscribe)
        .await
        .expect("grant event-subscribe");
    create_grant(handle.raw(), &admin(), &emitter, Capability::EventPublish)
        .await
        .expect("grant event-publish");

    // The watcher subscribes; the scope decision is taken once, here.
    let mut subscription =
        rubix_ext::subscribe_events(handle.raw(), &watcher, &bus, "rule.fired")
            .await
            .expect("granted extension may subscribe");

    // The emitter publishes an event threaded by a correlation id; it reaches the
    // one subscriber.
    let event = ControlEvent::new(
        "rule.fired",
        serde_json::json!({ "rule": "comfort@1" }),
        CorrelationId::carry("corr-bus-1"),
    );
    let reached = rubix_ext::publish_event(handle.raw(), &emitter, &bus, event.clone())
        .await
        .expect("granted extension may publish");
    assert_eq!(reached, 1, "the event reached the one subscriber");

    let received = subscription.recv().await.expect("subscriber receives event");
    assert_eq!(received, event);
}

#[tokio::test]
async fn an_ungranted_extension_is_denied_at_subscribe() {
    let handle = open_ext_store("ext_bus_subscribe_deny").await;
    let bus = ControlBus::new();

    // No `event-subscribe` grant conferred — the extension holds nothing.
    let extension = rubix_ext::register_extension(handle.raw(), "nosub-ext", TENANT, "k")
        .await
        .expect("register extension")
        .principal()
        .clone();

    // `ControlSubscription` is not `Debug`, so match rather than `expect_err`.
    match rubix_ext::subscribe_events(handle.raw(), &extension, &bus, "rule.fired").await {
        Ok(_) => panic!("an out-of-grant subscribe must be denied"),
        Err(err) => assert!(matches!(err, rubix_ext::ExtError::Denied(_))),
    }
}

#[tokio::test]
async fn an_ungranted_extension_is_denied_at_publish() {
    let handle = open_ext_store("ext_bus_publish_deny").await;
    let bus = ControlBus::new();

    // A subscriber is listening, so a leaked publish would be observable — but the
    // publisher holds no `event-publish` grant, so nothing fans out.
    let _live = rubix_bus::subscribe(&bus, "rule.fired");
    let extension = rubix_ext::register_extension(handle.raw(), "nopub-ext", TENANT, "k")
        .await
        .expect("register extension")
        .principal()
        .clone();

    let event = ControlEvent::new(
        "rule.fired",
        serde_json::json!({ "rule": "comfort@1" }),
        CorrelationId::carry("corr-bus-2"),
    );
    let err = rubix_ext::publish_event(handle.raw(), &extension, &bus, event)
        .await
        .expect_err("an out-of-grant publish must be denied");
    assert!(matches!(err, rubix_ext::ExtError::Denied(_)));
}
