//! `GET /records` — list the records visible to the principal's session.
//!
//! A read runs on the WS-03 scoped session: SurrealDB row-level permissions
//! return only the principal's namespace records (contract #1). On top of that
//! scope an optional `?kind=&tag=` filter narrows the result by collection and
//! tag set (`rubix/docs/design/BACKEND-COLLECTIONS.md`, "List/realtime filtering
//! by collection") — the grids that read a single collection ask for just that
//! kind. The filter only narrows; it cannot widen the session's scope. This is
//! also the surface a dashboard reads recorded insights through — insights are
//! generic records (`rubix-rules` records them as such), so they appear here
//! scoped to the principal.

use axum::Json;
use axum::extract::Query;
use rubix_gate::read_records_on_session_filtered;
use serde::Deserialize;

use crate::auth::Authenticated;
use crate::dto::record::RecordDto;
use crate::error::{ApiError, ApiResult};

/// Optional list filters parsed from the query string.
///
/// `kind` selects a collection; `tag` is a comma-separated set of tag names a
/// record must carry in full (Haystack-style intersection). Both are optional;
/// an absent or empty value omits that filter.
#[derive(Debug, Default, Deserialize)]
pub struct RecordListQuery {
    /// The collection kind to list (`content.kind`).
    kind: Option<String>,
    /// Comma-separated tag names the record must all carry.
    tag: Option<String>,
}

/// List the records the principal may read, optionally narrowed by kind/tag.
pub async fn list_records_route(
    auth: Authenticated,
    Query(query): Query<RecordListQuery>,
) -> ApiResult<Json<Vec<RecordDto>>> {
    let tags = parse_tags(query.tag.as_deref());
    let records = read_records_on_session_filtered(&auth.session, query.kind.as_deref(), &tags)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(records.into_iter().map(RecordDto::from).collect()))
}

/// Split a comma-separated `tag` value into trimmed, non-empty tag names.
fn parse_tags(raw: Option<&str>) -> Vec<String> {
    raw.into_iter()
        .flat_map(|s| s.split(','))
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::parse_tags;

    #[test]
    fn parse_tags_splits_trims_and_drops_empties() {
        assert_eq!(parse_tags(Some("hvac, floor-2 ,")), vec!["hvac", "floor-2"]);
        assert!(parse_tags(None).is_empty());
        assert!(parse_tags(Some("")).is_empty());
        assert!(parse_tags(Some("  ,  ")).is_empty());
    }
}
