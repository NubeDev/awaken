//! Open the Zenoh session and subscribe to the authorized key-space.
//!
//! The capability decision already happened in [`authorize`](super::authorize):
//! this verb takes the [`AuthorizedKeySpace`] it produced and declares a Zenoh
//! subscriber on exactly that scope (`rubix/docs/SCOPE.md`, "Ingestion and
//! pre-processing"). From here on the engine matches the resolved key expression
//! — the gate is never consulted again per message, so a high-rate stream stays
//! un-taxed (contract #2). Each received sample is decoded into the ingest-domain
//! [`Sample`](super::sample::Sample); a payload that is not valid JSON surfaces
//! as a decode error rather than being silently dropped or coerced.

use zenoh::Session;
use zenoh::handlers::FifoChannelHandler;
use zenoh::pubsub::Subscriber;
use zenoh::sample::Sample as ZenohSample;

use crate::error::{IngestError, Result};

use super::authorize::AuthorizedKeySpace;
use super::sample::Sample;

/// A live ingest subscription bound to one authorized key-space.
///
/// Holds the Zenoh session alongside the subscriber so the session is not closed
/// while samples are still arriving (Zenoh ties a subscriber's liveness to its
/// session). [`recv`](IngestSubscriber::recv) yields the next decoded sample.
pub struct IngestSubscriber {
    // Kept to own the session for the subscriber's lifetime; closing it would
    // tear the subscription down.
    _session: Session,
    subscriber: Subscriber<FifoChannelHandler<ZenohSample>>,
}

impl IngestSubscriber {
    /// Receive and decode the next sample on the subscribed key-space.
    ///
    /// Blocks (asynchronously) until a sample arrives. Decodes the payload as the
    /// free-form JSON content the platform persists.
    ///
    /// # Errors
    /// Returns [`IngestError::Session`] if the subscription channel has closed, or
    /// [`IngestError::Sample`] if the payload is not valid UTF-8 JSON.
    pub async fn recv(&self) -> Result<Sample> {
        let sample = self
            .subscriber
            .recv_async()
            .await
            .map_err(|e| IngestError::Session(format!("subscription closed: {e}")))?;
        decode_sample(&sample)
    }
}

/// Open a Zenoh peer session on `config` and subscribe to `authorized`.
///
/// The subscriber is declared on the already-authorized scope, so no key-space
/// decision is taken here. Returns the live [`IngestSubscriber`].
///
/// # Errors
/// Returns [`IngestError::Session`] if the session cannot be opened or the
/// subscriber cannot be declared.
pub async fn open_subscription(
    config: zenoh::Config,
    authorized: &AuthorizedKeySpace,
) -> Result<IngestSubscriber> {
    let session = zenoh::open(config)
        .await
        .map_err(|e| IngestError::Session(format!("open session: {e}")))?;
    let subscriber = session
        .declare_subscriber(authorized.scope().clone())
        .await
        .map_err(|e| IngestError::Session(format!("declare subscriber: {e}")))?;
    Ok(IngestSubscriber {
        _session: session,
        subscriber,
    })
}

/// Decode a raw Zenoh sample into the ingest-domain [`Sample`].
fn decode_sample(sample: &ZenohSample) -> Result<Sample> {
    let key = sample.key_expr().as_str().to_owned();
    let bytes = sample.payload().to_bytes();
    let content: serde_json::Value = serde_json::from_slice(&bytes)
        .map_err(|e| IngestError::Sample(format!("{key}: {e}")))?;
    Ok(Sample::new(key, content))
}
