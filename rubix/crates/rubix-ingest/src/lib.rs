//! Zenoh ingestion + in-flight pre-processing for the rubix platform.
//!
//! Streaming ingestion (`rubix/docs/SCOPE.md`, "Ingestion and pre-processing";
//! `rubix/STACK-DEISGN.md`, `rubix-ingest` row): sources publish to a Zenoh
//! fabric; the platform consumes the stream **in flight** — decimate, filter,
//! enrich — and only then persists. Raw high-rate streams are processed before
//! persistence, never written first and queried back.
//!
//! Two load-bearing contracts shape the crate:
//!
//! - **Key-space is one capability decision, taken at subscribe** (contract #2).
//!   [`authorize_keyspace`] consults the WS-04 gate exactly once to resolve the
//!   permitted key-space; [`open_subscription`] then declares the Zenoh
//!   subscriber on that resolved scope and the gate is never touched again per
//!   message, so a high-rate stream stays un-taxed. An out-of-grant key-space is
//!   refused at subscribe, before any session is opened.
//! - **Data is append-only, edge-partitioned** (contract #5). [`append_sample`]
//!   writes each surviving sample as a fresh record into the partition keyed by
//!   the principal's namespace (the edge identity), so two edges never write the
//!   same records and reconciliation is ordering + dedup, not merge.
//!
//! The crate is laid out one verb per file (`rubix/docs/FILE-LAYOUT.md`):
//! [`subscribe`] authorizes and opens the stream; [`process`] is the in-flight
//! decimate/filter/enrich pipeline; [`persist`] writes the edge-partitioned
//! append-only record.

mod error;
mod persist;
mod process;
mod subscribe;

pub use error::{IngestError, Result};
pub use persist::{INGEST_ROOT, append_sample, keyspace_root, partition_for};
pub use process::{Decimator, Enricher, Filter, Pipeline};
pub use subscribe::{
    AuthorizedKeySpace, IngestSubscriber, Sample, ZenohEndpoint, authorize_keyspace,
    open_subscription,
};
