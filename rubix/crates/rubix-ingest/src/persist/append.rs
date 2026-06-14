//! Write a pre-processed sample as an append-only record into the edge partition.
//!
//! Persistence is the last stage of ingest: a sample that survived
//! pre-processing is written into the partition keyed by the edge identity
//! (`rubix/STACK-DEISGN.md`, contract #5: append-only, edge-partitioned — two
//! edges never write the same records, so reconciliation is ordering + dedup, not
//! merge). The partition is the principal's namespace; the record is freshly
//! id-minted (ids are edge-mintable without coordination, `rubix/docs/SCOPE.md`),
//! so the write only ever *appends* — it never targets an existing row to update.
//!
//! The write does **not** re-cross the command gate per sample: the capability
//! decision was taken once at subscribe (`authorize`), so taxing every
//! high-rate message again would defeat the streaming design (contract #2). The
//! sample content is persisted as the free-form record content (principle 4); the
//! edge partition comes from the principal, never from the sample, so a publisher
//! cannot write into another edge's partition by spoofing a field.

use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use rubix_core::{Principal, Record, create_record};

use crate::error::{IngestError, Result};
use crate::subscribe::Sample;

use super::partition::partition_for;

/// Append `sample` as a new record in `principal`'s edge partition.
///
/// Builds a fresh-id [`Record`] in the partition namespace holding the sample's
/// content and creates it. Returns the persisted record (the round-trip confirms
/// the append landed).
///
/// # Errors
/// Returns [`IngestError::Persist`] if the append write fails.
pub async fn append_sample(
    db: &Surreal<Db>,
    principal: &Principal,
    sample: &Sample,
) -> Result<Record> {
    let partition = partition_for(principal);
    let record = Record::new(partition, sample.content.clone());
    create_record(db, &record)
        .await
        .map_err(|e| IngestError::Persist(e.to_string()))
}
