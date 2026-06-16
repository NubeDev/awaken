//! Integration: a published stream flows through pre-processing into the store.
//!
//! The headline ingest path (`rubix/docs/sessions/WS-12.md`, Done definition):
//! grant a key-space, open a local Zenoh peer session, publish samples to the
//! granted scope, and assert decimated/filtered records land append-only under
//! the edge partition. The capability decision is taken once at subscribe
//! (`authorize_keyspace`); from then on the engine matches the resolved scope and
//! the gate is never consulted per message.

#[path = "../fixture/mod.rs"]
mod fixture;

use rubix_ingest::{
    Decimator, Filter, Pipeline, append_sample, authorize_keyspace, open_subscription,
};

use fixture::open::{NS, granted_principal, open_ingest_store};
use fixture::peer::{listen_endpoint, open_publisher, publish, wait_linked};

const SUB_PORT: u16 = 17613;

// Zenoh's runtime requires a multi-thread scheduler; the default test flavor is
// current-thread.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn published_samples_flow_through_preprocessing_and_persist() {
    let handle = open_ingest_store("listen").await;
    let principal = granted_principal(&handle, "ingestor").await;

    // One capability decision, at subscribe.
    let authorized = authorize_keyspace(handle.raw(), &principal, "rubix/ingest/edge-7/**")
        .await
        .expect("authorize key-space");

    // Subscriber binds the loopback listen endpoint; publisher dials it.
    let config = listen_endpoint(SUB_PORT).to_config().expect("subscriber config");
    let subscriber = open_subscription(config, &authorized)
        .await
        .expect("open subscription");

    let publisher = open_publisher(SUB_PORT).await;
    wait_linked(&publisher).await;

    // Decimate one-in-two, drop out-of-range readings.
    let mut pipeline = Pipeline::new()
        .with_decimator(Decimator::new(2))
        .with_filter(Filter::new(|s| {
            s.content.get("value").and_then(serde_json::Value::as_f64).is_some_and(|t| t < 100.0)
        }));

    let temps = [20.0_f64, 21.0, 150.0, 22.0];
    for (n, value) in temps.into_iter().enumerate() {
        publish(
            &publisher,
            "rubix/ingest/edge-7/reg-temp",
            &serde_json::json!({ "n": n, "value": value }),
        )
        .await;
    }

    // Receive each published sample, pre-process, and persist the survivors.
    // Decimation keeps indices 0 and 2; index 2 (150.0) is filtered out, so a
    // single reading should land.
    let mut persisted = Vec::new();
    for _ in 0..temps.len() {
        let sample = tokio::time::timeout(std::time::Duration::from_secs(5), subscriber.recv())
            .await
            .expect("receive within timeout")
            .expect("decode sample");
        if let Some(survivor) = pipeline.admit(sample) {
            let reading = append_sample(handle.raw(), &principal, &survivor)
                .await
                .expect("append survivor");
            persisted.push(reading);
        }
    }

    assert_eq!(persisted.len(), 1, "decimate+filter should leave one survivor");
    let reading = &persisted[0];
    assert_eq!(reading.namespace, NS, "reading lands in the edge partition");
    assert_eq!(reading.series, "reg-temp", "series resolved from the key");
    assert!((reading.value - 20.0).abs() < f64::EPSILON);
}
