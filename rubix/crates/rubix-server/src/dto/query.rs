//! Wire shapes for the unified query surface.
//!
//! The transport DTO for the `POST /query` route (`rubix/docs/sessions/WS-16.md`):
//! a read-only SQL string in, the resulting rows out. The query runs through the
//! WS-04 `external-query` capability on the principal's scoped session
//! (`rubix-query::run_authorized`), so the rows are already bounded by SurrealDB
//! row-level permissions (contract #1).
//!
//! A request may carry a structured, UTC [`TimeScopeDto`]
//! (`rubix/docs/design/DASHBOARDS-SCOPE.md` §5): the board path sends absolute
//! epoch ms (or a relative token) plus a grain or target point count, and the
//! backend injects the window/bucket — never a locale datetime string spliced
//! client-side (the timezone bug this fixes).

use std::collections::HashMap;

use rubix_query::{Agg, CompareOp, Grain, QueryError, ReduceCalc, TimeBound, TimeScope, Transform};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

/// The body of a query request.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct QueryRequest {
    /// The read-only `SELECT`/`WITH` statement to run. May carry the time macros
    /// `$__timeFilter(col)`, `$__timeBucket(col)`, and `$__interval`, which the
    /// backend expands against `time`. Ignored when `query_id` is set.
    #[serde(default)]
    pub sql: String,
    /// An optional saved-query id (§4b). When set, the backend resolves the
    /// `kind:"query"` record's SQL **on the caller's scoped session** and runs it
    /// in place of `sql` — so a saved query never runs with the author's scope or
    /// caps, only the caller's (the §4b privilege-escalation guard).
    #[serde(default)]
    pub query_id: Option<String>,
    /// An optional structured, UTC time scope the backend injects (§5).
    #[serde(default)]
    pub time: Option<TimeScopeDto>,
    /// An optional `column → physical quantity` map (§2). After the rows are read
    /// (post-cache, per-caller), each named numeric column is converted from its
    /// canonical metric value to the requesting principal's unit system. A column
    /// not in the map is left untouched; the cache itself only holds raw values.
    #[serde(default)]
    pub quantities: Option<HashMap<String, String>>,
    /// An optional post-query transform pipeline (§1). The whole portable spec
    /// rides the request; the backend executes only the **aggregate** ops
    /// (`filter`/`groupBy`/`reduce`) over the result rows and leaves the cosmetic
    /// ops for the client. Empty/absent → rows pass through.
    #[serde(default)]
    pub transforms: Option<Vec<TransformDto>>,
}

/// One transform on the wire — a discriminated union mirroring the client spec
/// (§1). Only the aggregate variants are executed server-side; cosmetic variants
/// are accepted (so the contract stays whole) and skipped here.
#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum TransformDto {
    /// Cosmetic (client-side): copy `from` into `to`.
    Rename { from: String, to: String },
    /// Cosmetic (client-side): `field = left <op> right`.
    Calculated {
        field: String,
        left: String,
        op: String,
        right: String,
    },
    /// Aggregate (server-side): keep rows where `field <op> value`.
    Filter {
        field: String,
        op: String,
        value: String,
    },
    /// Aggregate (server-side): one row per `by`, aggregating `field` into `as`.
    GroupBy {
        by: String,
        field: String,
        agg: String,
        #[serde(rename = "as")]
        as_: String,
    },
    /// Aggregate (server-side): collapse all rows to one holding `calc(field)`.
    Reduce {
        field: String,
        calc: String,
        #[serde(rename = "as")]
        as_: String,
    },
    /// Cosmetic (client-side): reorder columns to follow `order`.
    Organize { order: Vec<String> },
}

impl TransformDto {
    /// Parse the wire transform into the query-layer [`Transform`].
    ///
    /// # Errors
    /// Returns [`QueryError::Rejected`] if an operator/aggregation/reduce token is
    /// not recognised.
    pub fn into_transform(self) -> Result<Transform, QueryError> {
        Ok(match self {
            TransformDto::Rename { from, to } => Transform::Rename { from, to },
            TransformDto::Calculated {
                field,
                left,
                op,
                right,
            } => Transform::Calculated {
                field,
                left,
                op,
                right,
            },
            TransformDto::Filter { field, op, value } => Transform::Filter {
                field,
                op: CompareOp::parse(&op)
                    .ok_or_else(|| QueryError::Rejected(format!("unknown filter op: {op}")))?,
                value,
            },
            TransformDto::GroupBy {
                by,
                field,
                agg,
                as_,
            } => Transform::GroupBy {
                by,
                field,
                agg: Agg::parse(&agg)
                    .ok_or_else(|| QueryError::Rejected(format!("unknown aggregation: {agg}")))?,
                as_,
            },
            TransformDto::Reduce { field, calc, as_ } => Transform::Reduce {
                field,
                calc: ReduceCalc::parse(&calc)
                    .ok_or_else(|| QueryError::Rejected(format!("unknown reduce calc: {calc}")))?,
                as_,
            },
            TransformDto::Organize { order } => Transform::Organize { order },
        })
    }
}

/// A structured, UTC time scope: window bounds plus an optional bucket grain.
///
/// `from`/`to` are absolute UTC epoch milliseconds **or** relative tokens
/// (`now`, `now-1h`, `now/d`), resolved server-side at request time. `grain` pins
/// an explicit bucket width; `target_points` asks the backend to snap a grain to
/// roughly that many buckets (ignored if `grain` is set). The backend owns both
/// the window injection and the interval snap — the client never recomputes them.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct TimeScopeDto {
    /// The inclusive lower bound: epoch ms (a JSON number) or a relative token.
    pub from: TimeBoundDto,
    /// The inclusive upper bound: epoch ms (a JSON number) or a relative token.
    pub to: TimeBoundDto,
    /// An explicit bucket grain.
    #[serde(default)]
    pub grain: Option<String>,
    /// A desired bucket count to snap a grain to (ignored if `grain` is set).
    #[serde(default)]
    pub target_points: Option<u32>,
}

/// One window bound on the wire: a number is epoch ms, a string is a token.
///
/// Accepting either keeps the board (relative tokens) and the console (absolute
/// instants) on the same field without a tagged enum on the wire.
#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(untagged)]
pub enum TimeBoundDto {
    /// An absolute UTC instant in epoch milliseconds.
    Absolute(i64),
    /// A relative token (`now`, `now-7d`, `now/d`) resolved server-side.
    Relative(String),
}

impl From<TimeBoundDto> for TimeBound {
    fn from(dto: TimeBoundDto) -> Self {
        match dto {
            TimeBoundDto::Absolute(ms) => TimeBound::Absolute(ms),
            TimeBoundDto::Relative(token) => TimeBound::Relative(token),
        }
    }
}

impl TimeScopeDto {
    /// Convert the wire scope into the query-layer [`TimeScope`], parsing the
    /// grain string.
    ///
    /// # Errors
    /// Returns [`QueryError::Rejected`] if `grain` is set but not a known grain.
    pub fn into_scope(self) -> Result<TimeScope, QueryError> {
        let grain = match self.grain {
            Some(ref raw) => Some(
                Grain::parse(raw)
                    .ok_or_else(|| QueryError::Rejected(format!("unknown grain: {raw}")))?,
            ),
            None => None,
        };
        Ok(TimeScope {
            from: self.from.into(),
            to: self.to.into(),
            grain,
            target_points: self.target_points,
        })
    }
}

/// The result of a query: the matched rows plus their column types.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct QueryResponse {
    /// The result rows, each a JSON object keyed by column name.
    pub rows: Vec<Value>,
    /// The result columns in order, so a client gets types without sniffing rows
    /// (feeds FieldConfig matching, §7).
    pub columns: Vec<ColumnDto>,
}

/// A result column's name and type, derived from the Arrow result schema.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ColumnDto {
    /// The column name.
    pub name: String,
    /// A coarse type tag (`number`/`string`/`boolean`/`timestamp`/`other`).
    #[serde(rename = "type")]
    pub kind: String,
}

/// The readable schema for the principal: tables + columns (§4b).
///
/// Backs query autocomplete and stops charts guessing the JSON shape. The tables
/// are exactly those the principal can read — native canonical tables (scoped
/// scan) plus external datasource tables when `external-query` is held.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct QuerySchemaResponse {
    /// Every readable table, ordered by schema then table name.
    pub tables: Vec<TableSchemaDto>,
}

/// One readable table and its columns, as addressed in SQL.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct TableSchemaDto {
    /// The schema the table lives under: the default catalog schema for native
    /// canonical tables, or a datasource id for external tables (addressed
    /// `"<schema>"."<table>"`).
    pub schema: String,
    /// The bare table name.
    pub table: String,
    /// The table's columns in declaration order, with the same coarse type tags
    /// as result columns ([`ColumnDto`]).
    pub columns: Vec<ColumnDto>,
}

/// The body of a batch query request: run a whole board in one round trip (§3).
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct BatchQueryRequest {
    /// The statements to run, each keyed so the client matches results to panels
    /// order-independently. Capped server-side.
    pub queries: Vec<BatchQueryItem>,
}

/// One keyed statement in a batch.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct BatchQueryItem {
    /// The caller's key for this statement (typically a chart id). Echoed back on
    /// the matching result.
    pub key: String,
    /// The read-only SQL, with the same time macros as `POST /query`. Ignored when
    /// `query_id` is set.
    #[serde(default)]
    pub sql: String,
    /// An optional saved-query id resolved on the caller's scope (§4b).
    #[serde(default)]
    pub query_id: Option<String>,
    /// An optional structured, UTC time scope injected into this statement.
    #[serde(default)]
    pub time: Option<TimeScopeDto>,
    /// An optional `column → physical quantity` map applied post-read (§2).
    #[serde(default)]
    pub quantities: Option<HashMap<String, String>>,
    /// An optional post-query transform pipeline; aggregate ops run server-side (§1).
    #[serde(default)]
    pub transforms: Option<Vec<TransformDto>>,
}

impl BatchQueryItem {
    /// The single-query request this item resolves to (so both paths share the
    /// time-macro resolution, the per-caller unit conversion, and the transforms).
    #[must_use]
    pub fn into_request(self) -> (String, QueryRequest) {
        (
            self.key,
            QueryRequest {
                sql: self.sql,
                query_id: self.query_id,
                time: self.time,
                quantities: self.quantities,
                transforms: self.transforms,
            },
        )
    }
}

/// The result of a batch query: one keyed result per statement.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct BatchQueryResponse {
    /// One result per input statement, matched to the input by `key`.
    pub results: Vec<BatchQueryResult>,
}

/// One statement's outcome: its rows + columns, or its error — never both.
///
/// A per-item error so one bad panel doesn't blank the board (§3); the HTTP status
/// stays `200` unless the request itself is malformed.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct BatchQueryResult {
    /// The caller's key for this statement.
    pub key: String,
    /// The rows, when the statement succeeded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rows: Option<Vec<Value>>,
    /// The columns, when the statement succeeded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub columns: Option<Vec<ColumnDto>>,
    /// The failure message, when the statement failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl BatchQueryResult {
    /// A successful result carrying rows + columns.
    #[must_use]
    pub fn ok(key: String, rows: Vec<Value>, columns: Vec<ColumnDto>) -> Self {
        Self {
            key,
            rows: Some(rows),
            columns: Some(columns),
            error: None,
        }
    }

    /// A failed result carrying the error message.
    #[must_use]
    pub fn failed(key: String, error: String) -> Self {
        Self {
            key,
            rows: None,
            columns: None,
            error: Some(error),
        }
    }
}
