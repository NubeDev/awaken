//! The decoded shape of one ingested Zenoh sample.
//!
//! A source publishes a sample to a key expression under the platform's ingest
//! root; the subscriber decodes its payload as the free-form JSON content the
//! platform persists (`rubix/docs/SCOPE.md`, principle 4: generic, not
//! domain-specific — structure comes from tags, not a baked-in schema). This is
//! the value the pre-processing nodes ([`decimate`](crate::process::decimate),
//! [`filter`](crate::process::filter), [`enrich`](crate::process::enrich))
//! transform in flight before persistence, so it is owned here and shared across
//! the subscribe and process modules rather than re-derived per stage.

/// One ingested sample: the key it arrived on plus its decoded JSON content.
///
/// `key` is the full Zenoh key expression the sample was published to (a
/// sub-space of the authorized scope); `content` is the free-form document the
/// payload decoded into. Pre-processing reads both — `key` is the only piece of
/// routing context a node has, and `content` is what it decimates/filters/
/// enriches before persistence.
#[derive(Debug, Clone, PartialEq)]
pub struct Sample {
    /// The Zenoh key expression the sample was published to.
    pub key: String,
    /// The free-form JSON content decoded from the sample payload.
    pub content: serde_json::Value,
}

impl Sample {
    /// Build a sample from its key and decoded content.
    #[must_use]
    pub fn new(key: impl Into<String>, content: serde_json::Value) -> Self {
        Self {
            key: key.into(),
            content,
        }
    }
}
