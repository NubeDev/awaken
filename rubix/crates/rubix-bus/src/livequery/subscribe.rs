//! Open a live-query subscription to a table on a scoped session.
//!
//! Contract #1 (`rubix/STACK-DEISGN.md`): the data-change plane runs on a
//! gate-issued **scoped SurrealDB session**, so SurrealDB row-level permissions
//! decide which records the subscriber sees — the scope is set once here at
//! subscribe, never proxied per message. The caller passes the scoped session's
//! own connection (`rubix_gate::ScopedSession::connection`); a subscription
//! opened on it inherits that session's permissions.

use surrealdb::Surreal;
use surrealdb::engine::local::Db;
use surrealdb::types::Value;

use crate::error::{BusError, Result};

use super::stream::DataChangeStream;

/// Subscribe to data changes on `table`, observed through `session`'s
/// permissions.
///
/// `session` is a principal-scoped connection (the gate's
/// `ScopedSession::connection`). The returned [`DataChangeStream`] yields a
/// [`DataChange`](super::DataChange) for every insert/update/delete to a record
/// in `table` that the session may read; records the principal cannot read
/// never produce an event, because the engine filters the live query at the
/// source.
///
/// # Errors
/// Returns [`BusError::Subscribe`] if the engine rejects the live query (e.g.
/// the session lacks live-query support or the table is undefined).
pub async fn subscribe_table(session: &Surreal<Db>, table: &str) -> Result<DataChangeStream> {
    let stream = session
        .select::<Vec<Value>>(table.to_owned())
        .live()
        .await
        .map_err(BusError::Subscribe)?;
    Ok(DataChangeStream::new(stream))
}
