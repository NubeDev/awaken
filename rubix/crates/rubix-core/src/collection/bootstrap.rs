//! Seed the bootstrap meta-collection: a collection that defines collections.
//!
//! PocketBase bootstraps with a collection-defining-collection
//! (`rubix/docs/design/BACKEND-COLLECTIONS.md`, open question 2); rubix does the
//! same. The meta-collection is a `kind: "collection"` record named `collection`,
//! so once strict mode is on, a malformed collection record is itself rejected by
//! the validate step against this contract. It is a platform built-in, not tenant
//! data, so it is written directly (like the gate/audit schema definitions at
//! boot) rather than as a gated command — there is no principal at boot.
//!
//! Seeding is idempotent: it is a no-op when the namespace already carries the
//! meta-collection, so re-running at every boot does not duplicate it or bump its
//! timestamps (which would emit spurious sync events).

use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use crate::error::Result;
use crate::id::Id;
use crate::record::{Record, create_record};

use super::def::COLLECTION_KIND;
use super::load::find_collection;

/// Build the deterministic id of a namespace's meta-collection record.
///
/// Records are keyed by id alone (namespace is a field), so the namespace is
/// folded into the id to keep one meta-collection per namespace without
/// collision across tenants.
fn meta_collection_id(namespace: &str) -> Id {
    Id::from_raw(format!("collection-meta-{namespace}"))
}

/// Ensure `namespace` carries the bootstrap meta-collection, creating it if absent.
///
/// The meta-collection declares the one field every collection record must
/// carry — a non-empty `name`. The richer fields (`schema`, `indexes`,
/// `listRule`) are intentionally left undeclared so validation stays lenient
/// about them (a collection's `schema` is a JSON array, which the closed field
/// enum does not model); the gate enforces only that a collection has a name.
///
/// # Errors
/// Returns [`Error::Store`](crate::Error::Store) if the existence check or the
/// create write fails.
pub async fn bootstrap_meta_collection(db: &Surreal<Db>, namespace: &str) -> Result<()> {
    if find_collection(db, namespace, COLLECTION_KIND)
        .await?
        .is_some()
    {
        return Ok(());
    }

    let content = serde_json::json!({
        "kind": COLLECTION_KIND,
        "name": COLLECTION_KIND,
        "schema": [
            { "name": "name", "type": "text", "required": true }
        ]
    });

    let mut record = Record::new(namespace, content);
    record.id = meta_collection_id(namespace);
    create_record(db, &record).await?;
    Ok(())
}
