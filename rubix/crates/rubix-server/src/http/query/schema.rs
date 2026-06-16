//! `GET /query/schema` — the tables + columns the principal can read (§4b).
//!
//! Backs query autocomplete and stops charts guessing the JSON shape
//! (`rubix/docs/design/DASHBOARDS-SCOPE.md` §4b). It is the shape-only twin of
//! `POST /query`: the **same** `external-query` capability, the **same** scoped
//! session, and the **same** scoped-context cache (§4a) — but it reports the
//! catalog instead of running a statement. Row-perm aware: a native table only
//! appears if the principal's scoped scan resolved, and external tables only when
//! the connector is registered and the caller holds `external-query`.

use axum::Json;
use axum::extract::State;
use rubix_datasource::{ColumnSchema, TableSchema, schema_of};

use crate::auth::Authenticated;
use crate::dto::query::{ColumnDto, QuerySchemaResponse, TableSchemaDto};
use crate::error::ApiResult;
use crate::http::query::run::map_query_error;
use crate::state::AppState;

/// Return the readable schema for the requesting principal.
///
/// A missing `external-query` grant is `403`; an engine failure building the
/// context is `500`. Tables are returned in catalog order (schema, then table).
pub async fn query_schema_route(
    State(state): State<AppState>,
    auth: Authenticated,
) -> ApiResult<Json<QuerySchemaResponse>> {
    let tables = schema_of(
        &*state.datasources.read().await,
        state.store.raw(),
        &auth.session,
        &state.context_cache,
    )
    .await
    .map_err(map_query_error)?;

    Ok(Json(QuerySchemaResponse {
        tables: tables.into_iter().map(into_dto).collect(),
    }))
}

/// Convert a datasource [`TableSchema`] into its wire DTO.
fn into_dto(table: TableSchema) -> TableSchemaDto {
    TableSchemaDto {
        schema: table.schema,
        table: table.table,
        columns: table.columns.into_iter().map(column_dto).collect(),
    }
}

/// Convert a datasource [`ColumnSchema`] into the shared result-column DTO.
fn column_dto(column: ColumnSchema) -> ColumnDto {
    ColumnDto {
        name: column.name,
        kind: column.kind.to_owned(),
    }
}
