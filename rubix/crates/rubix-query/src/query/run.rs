//! Execute a read-only SQL query through DataFusion over the scoped tables.
//!
//! The unified query entry: guard the statement, build a DataFusion context whose
//! canonical tables were scanned through the principal's scoped session, plan and
//! run the SQL, and return the result rows. DataFusion sits above SurrealDB only
//! for this unification/aggregation step (`rubix/STACK-DEISGN.md`, contract #6);
//! the row read and its permission scope are SurrealDB's (contract #1). The
//! capability check is the caller's responsibility through [`run_authorized`]
//! (see [`super::authorize`]), so the raw [`run`] stays a pure execution verb.

use datafusion::arrow::record_batch::RecordBatch;
use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use crate::error::Result;
use crate::provider::build_context;

use super::guard::ensure_read_only;

/// Run a single read-only `SELECT`/`WITH` query over the canonical tables.
///
/// `session` is a gate-issued scoped connection; the visible rows are the ones
/// its SurrealDB permissions admit. The statement is guarded first — a non-read
/// or multi-statement input is rejected before any scan (see
/// [`ensure_read_only`]) — then planned and executed in DataFusion. Results come
/// back as Arrow [`RecordBatch`]es (possibly empty), the engine's native row
/// shape, which the transport layer (WS-16) renders to the wire.
///
/// This verb does **not** check the query capability; callers that enforce
/// contract #2 use [`run_authorized`](super::authorize::run_authorized).
///
/// # Errors
/// Returns [`QueryError::Rejected`](crate::QueryError::Rejected) if the statement
/// is not a single read-only query, or a scan/DataFusion error if execution
/// fails.
pub async fn run(session: &Surreal<Db>, sql: &str) -> Result<Vec<RecordBatch>> {
    ensure_read_only(sql)?;
    let ctx = build_context(session).await?;
    let dataframe = ctx.sql(sql).await?;
    let batches = dataframe.collect().await?;
    Ok(batches)
}
