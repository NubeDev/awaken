//! `GET /rules` — list the rules visible to the principal's session.
//!
//! A read runs on the WS-03 scoped session: SurrealDB row-level permissions return
//! only the principal's namespace rules (contract #1). The handler reads the
//! `kind:"rule"` collection and projects each record to its rule DTO, dropping any
//! record whose content is not a well-formed rule document.

use axum::Json;

use crate::auth::Authenticated;
use crate::dto::rule::RuleDto;
use crate::error::ApiResult;
use crate::http::rules::shared::read_rules;

/// List the rules the principal may read.
pub async fn list_rules_route(auth: Authenticated) -> ApiResult<Json<Vec<RuleDto>>> {
    Ok(Json(read_rules(&auth.session).await?))
}
