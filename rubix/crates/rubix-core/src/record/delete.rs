//! DELETE a generic record and clear its tag edges.

use surrealdb::Surreal;
use surrealdb::engine::local::Db;
use surrealdb::types::RecordId;

use crate::error::{Error, Result};
use crate::id::Id;
use crate::tag::TAGGED_EDGE;

use super::RECORD_TABLE;

/// Delete `record:<id>` and every `tagged` edge leaving it.
///
/// SurrealDB does not cascade graph-edge deletes, so a dangling `tagged` edge
/// would otherwise outlive its record and pollute tag-set traversals. Both
/// deletes run in one statement batch so the record never lingers with orphaned
/// edges (contract #6: graph + document in the one engine).
///
/// # Errors
/// Returns [`Error::Store`] if either delete fails.
pub async fn delete_record(db: &Surreal<Db>, id: &Id) -> Result<()> {
    let thing = RecordId::new(RECORD_TABLE, id.as_str());
    db.query(format!("DELETE {TAGGED_EDGE} WHERE in = $record"))
        .query("DELETE $record")
        .bind(("record", thing))
        .await
        .map_err(|e| Error::Store(e.to_string()))?
        .check()
        .map_err(|e| Error::Store(e.to_string()))?;
    Ok(())
}
