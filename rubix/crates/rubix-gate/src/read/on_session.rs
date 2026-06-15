//! Run a scoped read on a principal's session.
//!
//! This is the read enforcement point in action: the verbs issue plain `SELECT`s
//! over the principal's [`ScopedSession`] and SurrealDB row-level permissions
//! return only the principal's namespace records. There is no app filter here —
//! proving the scope is engine-native (contract #2, `rubix/STACK-DEISGN.md`).
//! Reads are not proxied per message; they run directly on the session the gate
//! issued once (contract #1).

use rubix_core::{Id, Record, list_records, list_records_filtered, read_record};

use crate::error::{GateError, Result};
use crate::session::ScopedSession;

/// Read the records visible to `session`'s principal.
///
/// Returns only the principal's namespace data, enforced by SurrealDB — a
/// principal in namespace A never sees namespace B's records through this path.
///
/// # Errors
/// Returns [`GateError::Read`] if the underlying scoped query fails.
pub async fn read_records_on_session(session: &ScopedSession) -> Result<Vec<Record>> {
    list_records(session.connection())
        .await
        .map_err(GateError::Read)
}

/// Read the principal's records narrowed by collection `kind` and/or `tag` set.
///
/// The filter runs on the same scoped session, so SurrealDB row-level
/// permissions still bound the result first; `kind`/`tags` only narrow within
/// that scope (`rubix/docs/design/BACKEND-COLLECTIONS.md`, "List/realtime
/// filtering by collection"). `None`/empty filters make this equivalent to
/// [`read_records_on_session`].
///
/// # Errors
/// Returns [`GateError::Read`] if the underlying scoped query fails.
pub async fn read_records_on_session_filtered(
    session: &ScopedSession,
    kind: Option<&str>,
    tags: &[String],
) -> Result<Vec<Record>> {
    list_records_filtered(session.connection(), kind, tags)
        .await
        .map_err(GateError::Read)
}

/// Read a single record by id on `session`, or `None` if the principal may not
/// see it.
///
/// A record outside the principal's namespace resolves to `None` because the
/// engine's row-level permission excludes it from the `select` — the denial is
/// SurrealDB-native, not an app check.
///
/// # Errors
/// Returns [`GateError::Read`] if the underlying scoped query fails.
pub async fn read_record_on_session(session: &ScopedSession, id: &Id) -> Result<Option<Record>> {
    read_record(session.connection(), id)
        .await
        .map_err(GateError::Read)
}
