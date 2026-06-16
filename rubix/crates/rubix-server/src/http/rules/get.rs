//! `GET /rules/:name` — read one rule on the principal's scoped session.
//!
//! A read runs on the WS-03 scoped session, so a rule outside the principal's
//! namespace resolves to `404` natively (contract #1). Rules are addressed by
//! their *name* — the composition handle — not their storage id; the handler loads
//! the rule by name on the session.

use axum::Json;
use axum::extract::Path;

use crate::auth::Authenticated;
use crate::dto::rule::RuleDto;
use crate::error::ApiResult;
use crate::http::rules::shared::read_rule_by_name;

/// Read the rule named `name` if the principal's session may see it, else `404`.
pub async fn get_rule_route(
    auth: Authenticated,
    Path(name): Path<String>,
) -> ApiResult<Json<RuleDto>> {
    Ok(Json(read_rule_by_name(&auth.session, &name).await?))
}
