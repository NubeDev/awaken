//! DELETE a tag node and clear its inbound edges.

use surrealdb::Surreal;
use surrealdb::engine::local::Db;
use surrealdb::types::RecordId;

use crate::error::{Error, Result};
use crate::id::Id;

use super::{TAG_TABLE, TAGGED_EDGE};

/// Delete `tag:<id>` and every `tagged` edge pointing at it.
///
/// As with record delete, SurrealDB does not cascade edge deletes, so the
/// inbound `tagged` edges are removed first to avoid dangling edges in tag-set
/// traversals.
///
/// # Errors
/// Returns [`Error::Store`] if either delete fails.
pub async fn delete_tag(db: &Surreal<Db>, id: &Id) -> Result<()> {
    let thing = RecordId::new(TAG_TABLE, id.as_str());
    db.query(format!("DELETE {TAGGED_EDGE} WHERE out = $tag"))
        .query("DELETE $tag")
        .bind(("tag", thing))
        .await
        .map_err(|e| Error::Store(e.to_string()))?
        .check()
        .map_err(|e| Error::Store(e.to_string()))?;
    Ok(())
}
