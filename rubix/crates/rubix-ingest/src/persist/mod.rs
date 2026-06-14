//! Append-only, edge-partitioned persistence of ingested samples.
//!
//! A pre-processed sample is [`append`]ed as a fresh record into the partition
//! keyed by the edge identity ([`partition`]), so two edges never write the same
//! records (`rubix/STACK-DEISGN.md`, contract #5).

mod append;
mod partition;

pub use append::append_sample;
pub use partition::{INGEST_ROOT, keyspace_root, partition_for};
