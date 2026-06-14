//! The thin SQL-execution seam.
//!
//! `SqlBackend` is the only trait that touches a live database. Everything else
//! in the crate — single-statement validation, parameter binding, caps,
//! named-query lookup, the result shape — sits above it and is unit-testable
//! against a fake implementation, so the cap/param/single-statement/named-query
//! logic needs no live DB to test (the prompt's testability requirement). The
//! sqlx Postgres implementation lives in [`super::postgres`].
//!
//! The trait deliberately speaks in the crate's own vocabulary (validated SQL,
//! typed [`Param`]s, a wall-clock [`std::time::Duration`]) rather than sqlx
//! types, so the wall-clock cap is "the engine's native mechanism" (Postgres
//! `statement_timeout`) phrased so a non-Postgres backend could slot in later
//! (docs "Wall-clock cap").

use std::time::Duration;

use async_trait::async_trait;

use super::rows::Column;
use crate::error::DatasourceResult;
use crate::statement::Params;

/// One executed read's raw output before caps are applied above it: the columns
/// the engine reported and the decoded JSON rows. Byte/row caps are enforced by
/// the executor; `fetch_bound` lets the backend also cap the pull server-side.
#[derive(Debug, Clone, PartialEq)]
pub struct RawResult {
    pub columns: Vec<Column>,
    pub rows: Vec<crate::backend::Row>,
}

/// A read-only execution surface over one external SQL engine.
///
/// Implementors MUST:
/// - bind `params` positionally as `$1..$N`, never splice them into `sql`,
/// - apply `wall_clock` as the engine's native statement timeout when set,
/// - fetch at most `fetch_bound` rows when set (so an unbounded result cannot be
///   pulled into memory before the executor's caps apply),
/// - never issue a write and never log credentials.
#[async_trait]
pub trait SqlBackend: Send + Sync {
    /// Run exactly one already-validated statement and return its raw rows.
    async fn run(
        &self,
        sql: &str,
        params: &Params,
        wall_clock: Option<Duration>,
        fetch_bound: Option<u64>,
    ) -> DatasourceResult<RawResult>;

    /// Introspect the columns this datasource exposes via `information_schema`
    /// under the read-only role (docs "Schema discovery"). Returns one row per
    /// `(table, column, type)`; the describe layer shapes it.
    async fn introspect(&self) -> DatasourceResult<RawResult>;
}
