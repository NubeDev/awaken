//! Append-only, edge-partitioned persistence of ingested samples.
//!
//! A pre-processed sample is [`append`]ed as a [`Reading`](rubix_core::Reading)
//! into the `reading` data plane, in the partition keyed by the edge identity
//! ([`partition`]), so two edges never write the same rows
//! (`rubix/STACK-DEISGN.md`, contract #5; `rubix/docs/design/READINGS-TIMESERIES.md`).

mod append;
mod partition;

pub use append::append_sample;
pub use partition::{INGEST_ROOT, keyspace_root, partition_for};
