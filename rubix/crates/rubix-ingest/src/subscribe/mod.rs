//! Zenoh ingest subscription — scoped once at subscribe by the gate.
//!
//! The capability decision is taken exactly once ([`authorize`]) so high-rate
//! streams stay un-taxed; [`listen`] then declares the subscriber on the
//! resolved scope and yields decoded [`Sample`](sample::Sample)s. The
//! [`endpoint`] config selects how the peer session reaches the fabric.

mod authorize;
mod endpoint;
mod listen;
mod sample;

pub use authorize::{AuthorizedKeySpace, authorize_keyspace};
pub use endpoint::ZenohEndpoint;
pub use listen::{IngestSubscriber, open_subscription};
pub use sample::Sample;
