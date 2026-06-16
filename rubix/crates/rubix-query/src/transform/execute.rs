//! Execute the aggregate transform tier as a small DataFusion stage (§1).
//!
//! The aggregate ops (`filter`/`groupBy`/`reduce`) need the full result set and
//! shrink the wire, so they run server-side (`rubix/docs/design/DASHBOARDS-SCOPE.md`
//! §1, the hybrid decision). Rather than a hand-rolled row engine, each op is run
//! through DataFusion over the result batches: the rows are registered as an
//! in-memory table and a generated `SELECT` re-derives them, chained step by step.
//! This reuses the engine that already produced the rows and keeps the stage
//! exactly as powerful as the SQL it generates — no more.
//!
//! ⚠ The generated SQL embeds column names, so every identifier is validated
//! against a strict charset and quoted before it reaches a query string; a
//! filter's literal is bound as a single-quoted string literal with quotes
//! escaped. Cosmetic ops are skipped here (the client runs them), so this stage
//! only ever generates aggregation/filter SQL over one trusted in-memory table.

use std::sync::Arc;

use datafusion::arrow::record_batch::RecordBatch;
use datafusion::datasource::MemTable;
use datafusion::prelude::SessionContext;

use crate::error::{QueryError, Result};

use super::spec::{Agg, ReduceCalc, Transform};

/// The name the working relation is registered under at each pipeline step.
const RELATION: &str = "_transform_input";

/// Apply the aggregate transforms in `transforms` to `batches`, in order.
///
/// Cosmetic transforms are skipped (the client executes them). With no aggregate
/// transform the input is returned unchanged. Each aggregate step registers the
/// current rows as an in-memory table and runs a generated `SELECT` to produce the
/// next rows, so the steps compose without a bespoke row engine.
///
/// # Errors
/// Returns [`QueryError::Rejected`] if a transform names an invalid identifier (so
/// a bad spec is a clean rejection, never injected SQL), or
/// [`QueryError::DataFusion`] if a generated statement fails to plan or execute.
pub async fn apply_aggregate_transforms(
    batches: Vec<RecordBatch>,
    transforms: &[Transform],
) -> Result<Vec<RecordBatch>> {
    if !transforms.iter().any(Transform::is_aggregate) {
        return Ok(batches);
    }
    // An empty result has no schema to register; aggregates over it stay empty.
    let Some(first) = batches.first() else {
        return Ok(batches);
    };
    let schema = first.schema();

    let mut current = batches;
    for transform in transforms {
        if !transform.is_aggregate() {
            continue; // cosmetic — the client runs it
        }
        // Register the working rows under their own schema (or the original schema
        // if a prior step emptied them) so the next SELECT plans against the right
        // columns.
        let step_schema = current
            .first()
            .map_or_else(|| Arc::clone(&schema), RecordBatch::schema);
        let ctx = SessionContext::new();
        let provider = MemTable::try_new(step_schema, vec![current])
            .map_err(QueryError::DataFusion)?;
        ctx.register_table(RELATION, Arc::new(provider))
            .map_err(QueryError::DataFusion)?;
        let sql = generate_sql(transform)?;
        current = ctx.sql(&sql).await?.collect().await?;
    }
    Ok(current)
}

/// Generate the `SELECT` that realises one aggregate transform over [`RELATION`].
fn generate_sql(transform: &Transform) -> Result<String> {
    match transform {
        Transform::Filter { field, op, value } => {
            let col = quote_ident(field)?;
            let literal = filter_literal(value);
            Ok(format!(
                "SELECT * FROM {RELATION} WHERE {col} {} {literal}",
                op.sql()
            ))
        }
        Transform::GroupBy {
            by,
            field,
            agg,
            as_,
        } => {
            let by_col = quote_ident(by)?;
            let as_col = quote_ident(as_)?;
            let agg_expr = aggregate_expr(*agg, field)?;
            Ok(format!(
                "SELECT {by_col}, {agg_expr} AS {as_col} FROM {RELATION} \
                 GROUP BY {by_col} ORDER BY {by_col}"
            ))
        }
        Transform::Reduce { field, calc, as_ } => {
            let as_col = quote_ident(as_)?;
            Ok(reduce_sql(*calc, field, &as_col)?)
        }
        // Cosmetic ops never reach here (callers skip them); be explicit.
        _ => Err(QueryError::Rejected(
            "non-aggregate transform has no server-side SQL".to_owned(),
        )),
    }
}

/// The aggregate column expression, e.g. `sum("temp")`. `count` counts rows
/// (`count(*)`) so it works regardless of the column's type.
fn aggregate_expr(agg: Agg, field: &str) -> Result<String> {
    Ok(match agg {
        Agg::Count => "count(*)".to_owned(),
        other => format!("{}({})", other.sql_func(), quote_ident(field)?),
    })
}

/// Generate the single-row `SELECT` for a reduce calc.
///
/// `first`/`last` need a stable order; the result rows carry no inherent order, so
/// they reduce to an arbitrary-but-deterministic `min`/`max`-free pick via
/// `first_value`/`last_value` over the unordered relation is not meaningful — so
/// `first`/`last` fall back to taking any single row's value with `LIMIT 1`. The
/// numeric calcs aggregate the whole column.
fn reduce_sql(calc: ReduceCalc, field: &str, as_col: &str) -> Result<String> {
    let col = quote_ident(field)?;
    Ok(match calc {
        ReduceCalc::Count => format!("SELECT count(*) AS {as_col} FROM {RELATION}"),
        ReduceCalc::Sum => format!("SELECT sum({col}) AS {as_col} FROM {RELATION}"),
        ReduceCalc::Avg => format!("SELECT avg({col}) AS {as_col} FROM {RELATION}"),
        ReduceCalc::Min => format!("SELECT min({col}) AS {as_col} FROM {RELATION}"),
        ReduceCalc::Max => format!("SELECT max({col}) AS {as_col} FROM {RELATION}"),
        // No inherent row order on result batches, so first/last take one row.
        ReduceCalc::First | ReduceCalc::Last => {
            format!("SELECT {col} AS {as_col} FROM {RELATION} LIMIT 1")
        }
    })
}

/// Validate and double-quote a SQL identifier.
///
/// Only ASCII alphanumerics and `_` are allowed (the shape every canonical/result
/// column has), so a quoted identifier can never break out of its quotes — a bad
/// name is rejected, never embedded. Empty names are rejected too.
fn quote_ident(name: &str) -> Result<String> {
    if name.is_empty() || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
        return Err(QueryError::Rejected(format!(
            "invalid transform column identifier: {name:?}"
        )));
    }
    Ok(format!("\"{name}\""))
}

/// Render a filter literal for SQL, mirroring the client filter's typing rule.
///
/// The client compares numerically when the value parses as a finite number and
/// as strings otherwise. So a numeric value becomes a **bare** SQL numeric literal
/// (DataFusion then compares the column numerically, coercing as needed), and any
/// other value becomes a single-quoted string with embedded quotes escaped so it
/// can never break out.
fn filter_literal(value: &str) -> String {
    match value.parse::<f64>() {
        Ok(n) if n.is_finite() => value.to_owned(),
        _ => format!("'{}'", value.replace('\'', "''")),
    }
}
