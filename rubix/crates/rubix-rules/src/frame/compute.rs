//! The shared compute path every primitive funnels through.
//!
//! Each primitive builds a SQL statement over the registered input table and
//! calls [`Frame::compute`]. Concentrating execution here means the
//! no-row-explosion invariant is enforced in exactly one place: whatever SQL a
//! primitive emits, the result is rejected if it has more rows than the input.

use std::sync::Arc;

use datafusion::arrow::record_batch::RecordBatch;
use datafusion::datasource::MemTable;
use datafusion::prelude::SessionContext;

use super::{Frame, TABLE};
use crate::error::RuleError;

impl Frame {
    /// Run one SQL statement over this frame's rows and return the result frame.
    ///
    /// The frame's batches are registered as the in-memory table [`TABLE`] in a
    /// fresh `SessionContext` (one engine per call, no shared state), `sql` is
    /// planned and executed, and the collected batches become a new `Frame`.
    ///
    /// Enforces the hard invariant: a primitive may never increase row count.
    /// The curated surface only emits projections / filters / aggregates over
    /// the single table — no joins — so this guard should never trip; it is a
    /// belt-and-braces backstop because a runaway row count on a scheduled rule
    /// is the failure the sandbox's size limits cannot otherwise catch.
    pub(crate) fn compute(&self, sql: &str) -> Result<Frame, RuleError> {
        let batches = self.batches.clone();
        let schema = self.schema.clone();
        let input_rows = self.row_count();
        let sql = sql.to_string();

        let result = run_blocking(move || async move {
            let ctx = SessionContext::new();
            let table = MemTable::try_new(schema, vec![(*batches).clone()])?;
            ctx.register_table(TABLE, Arc::new(table))?;
            let df = ctx.sql(&sql).await?;
            let out_schema = df.schema().inner().clone();
            let out = df.collect().await?;
            Ok::<_, RuleError>((out_schema, out))
        })?;

        let (out_schema, out_batches) = result;
        let out_rows: usize = out_batches.iter().map(RecordBatch::num_rows).sum();
        if out_rows > input_rows {
            return Err(RuleError::Engine(format!(
                "primitive increased row count {input_rows} -> {out_rows}; \
                 no primitive may grow the frame"
            )));
        }

        Ok(Frame::new(out_schema, out_batches))
    }
}

impl Frame {
    /// Run `sql` over this frame without the no-growth guard.
    ///
    /// For *reducing* primitives whose output shape is fixed-and-small by
    /// construction (a scalar aggregate row, or one row per column) rather than
    /// bounded by the input row count — e.g. `describe`, `any_true`. These can
    /// emit one row from zero input rows, which the row-count guard would
    /// (wrongly) read as growth. They cannot explode: an aggregate without a
    /// large group-by key produces at most a handful of rows.
    pub(crate) fn compute_reduce(&self, sql: &str) -> Result<Frame, RuleError> {
        let batches = self.batches.clone();
        let schema = self.schema.clone();
        let sql = sql.to_string();
        let (out_schema, out) = run_blocking(move || async move {
            let ctx = SessionContext::new();
            let table = MemTable::try_new(schema, vec![(*batches).clone()])?;
            ctx.register_table(TABLE, Arc::new(table))?;
            let df = ctx.sql(&sql).await?;
            let s = df.schema().inner().clone();
            Ok::<_, RuleError>((s, df.collect().await?))
        })?;
        Ok(Frame::new(out_schema, out))
    }
}

/// Double-quote a SQL identifier (column name), escaping embedded quotes.
///
/// Column names reach SQL from the script, so they are quoted rather than
/// interpolated raw — a name like `a"; drop` becomes a harmless quoted ident,
/// not a second statement. Primitives use this for every column reference.
pub(crate) fn quote_ident(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

/// Reject a column name the script asked for but the frame does not have.
///
/// Turns a typo into a clean [`RuleError::Runtime`] instead of an opaque planner
/// error, and confirms the column exists before it is spliced into SQL.
pub(crate) fn require_column(frame: &Frame, col: &str) -> Result<(), RuleError> {
    if frame.schema().column_with_name(col).is_some() {
        Ok(())
    } else {
        Err(RuleError::Runtime(format!("no such column `{col}`")))
    }
}

/// Block on `f` using a fresh current-thread runtime.
///
/// Rhai is synchronous; DataFusion is async. A current-thread runtime per
/// primitive call is cheap and keeps execution single-threaded and free of
/// cross-tenant async state. Builds inside a `spawn_blocking`-free path because
/// primitives are already called from Rhai's (sync) thread.
fn run_blocking<F, Fut, T>(f: F) -> Result<T, RuleError>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<T, RuleError>>,
{
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| RuleError::Engine(format!("build runtime: {e}")))?;
    rt.block_on(f())
}
