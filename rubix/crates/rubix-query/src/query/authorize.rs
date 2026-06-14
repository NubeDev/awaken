//! Gate the query action on a WS-04 capability before running it.
//!
//! Querying the unified surface is an **app-enforced capability**, not a
//! SurrealDB row permission (`rubix/STACK-DEISGN.md`, contract #2; the second
//! authz layer of `rubix/docs/SCOPE.md`). A principal may read records its scoped
//! session permits, but exercising the DataFusion query/compute plane requires
//! the [`Capability::ExternalQuery`] grant on its principal. This verb makes that
//! check first and runs the query only if it is granted — fail closed: a missing
//! grant denies, and an error in the grant lookup is surfaced, never read as
//! allow.

use datafusion::arrow::record_batch::RecordBatch;
use rubix_gate::{Capability, ScopedSession, check_capability};
use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use crate::error::{QueryError, Result};

use super::run::run;

/// The capability the query action requires.
///
/// The DataFusion surface unifies SurrealDB with external datasources, so
/// exercising it is the `external-query` grant — the cross-plane query capability
/// the gate already registers (WS-04).
const QUERY_CAPABILITY: Capability = Capability::ExternalQuery;

/// Run `sql` for the principal of `session`, only if it holds the query grant.
///
/// `grant_reader` is a connection that can read the `grant` table — the store
/// root handle's connection — because grants carry no scoped-session `select`
/// permission (they are app-enforced, WS-04). `session` is the principal's scoped
/// read session: the grant is checked against its principal, and the query then
/// runs through it so SurrealDB row-level permissions still bound the rows.
///
/// # Errors
/// Returns [`QueryError::Denied`] if the principal lacks the query capability,
/// [`QueryError::Capability`] if the grant lookup itself fails, or a
/// guard/scan/DataFusion error from [`run`].
pub async fn run_authorized(
    grant_reader: &Surreal<Db>,
    session: &ScopedSession,
    sql: &str,
) -> Result<Vec<RecordBatch>> {
    let granted = check_capability(grant_reader, session.principal(), QUERY_CAPABILITY)
        .await
        .map_err(|e| QueryError::Capability(e.to_string()))?;
    if !granted {
        return Err(QueryError::Denied);
    }
    run(session.connection(), sql).await
}
