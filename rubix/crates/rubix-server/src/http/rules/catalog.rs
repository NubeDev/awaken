//! `GET /rules/catalog?table=…` — discover what a table offers a binding.
//!
//! Authoring a binding means naming a numeric series and an optional key/value to
//! narrow it by — facts the author cannot see from the editor. This surface
//! introspects the table the binding reads and reports the bindable fields and the
//! distinct filter values, so the studio offers them instead of making the author
//! type a series id or `content` key blind.
//!
//! Like the dry-run it backs, this is a read in effect on the principal's own
//! visible rows: it runs on the WS-03 scoped session (SurrealDB row-level
//! permissions decide what is discoverable, contract #1) and records nothing, so
//! it does not gate on [`RuleDefine`](rubix_gate::Capability::RuleDefine) —
//! exploring one's own data is not a mutation.

use axum::Json;
use axum::extract::Query;
use rubix_query::{FilterFacet, discover_facets};
use serde::Deserialize;

use crate::auth::Authenticated;
use crate::dto::rule::{CatalogResponse, FilterFacetDto, parse_table};
use crate::error::{ApiError, ApiResult};

/// The `?table=` selector naming which canonical table to discover.
#[derive(Debug, Deserialize)]
pub struct CatalogQuery {
    /// The canonical table name (`readings`, `records`, …) to introspect.
    table: String,
}

/// Discover the bindable facets of `table` for the requesting principal.
///
/// An unknown or missing `table` is a `400` (the binding could never resolve it);
/// a scoped-scan failure is a `500`. The facets are bounded by the rows the
/// principal may read.
pub async fn catalog_route(
    auth: Authenticated,
    Query(query): Query<CatalogQuery>,
) -> ApiResult<Json<CatalogResponse>> {
    let table = parse_table(&query.table).map_err(ApiError::BadRequest)?;

    let facets = discover_facets(auth.session.connection(), table)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(CatalogResponse {
        table: query.table,
        fields: facets.fields,
        filters: facets.filters.into_iter().map(filter_dto).collect(),
    }))
}

/// Project a discovered [`FilterFacet`] onto its wire DTO.
fn filter_dto(facet: FilterFacet) -> FilterFacetDto {
    FilterFacetDto {
        key: facet.key,
        values: facet.values,
        truncated: facet.truncated,
    }
}
