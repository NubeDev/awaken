//! Wire shapes for the unified query surface.
//!
//! The transport DTO for the `POST /query` route (`rubix/docs/sessions/WS-16.md`):
//! a read-only SQL string in, the resulting rows out. The query runs through the
//! WS-04 `external-query` capability on the principal's scoped session
//! (`rubix-query::run_authorized`), so the rows are already bounded by SurrealDB
//! row-level permissions (contract #1).

use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

/// The body of a query request.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct QueryRequest {
    /// The read-only `SELECT`/`WITH` statement to run.
    pub sql: String,
}

/// The result of a query: the matched rows as JSON objects.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct QueryResponse {
    /// The result rows, each a JSON object keyed by column name.
    pub rows: Vec<Value>,
}
