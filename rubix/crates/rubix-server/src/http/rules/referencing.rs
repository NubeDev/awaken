//! `GET /rules/:name/referencing` — the rules that compose this one.
//!
//! Editing or deleting a rule changes every rule that `invoke`s it on the next
//! tick, so the change-impact (blast-radius) surface is shown before a
//! behaviour-changing action (`rubix/docs/SCOPE.md`, the rules safety feature).
//! This read runs on the WS-03 scoped session and returns the visible rules whose
//! `subrules` name `name` — the set that would break if it changed.

use axum::Json;
use axum::extract::Path;

use crate::auth::Authenticated;
use crate::dto::rule::RuleDto;
use crate::error::ApiResult;
use crate::http::rules::shared::read_rules;

/// List the rules that compose the rule named `name` (reference it via `invoke`).
pub async fn referencing_rules_route(
    auth: Authenticated,
    Path(name): Path<String>,
) -> ApiResult<Json<Vec<RuleDto>>> {
    let referencing = read_rules(&auth.session)
        .await?
        .into_iter()
        .filter(|rule| rule.name != name && rule.subrules.contains(&name))
        .collect();
    Ok(Json(referencing))
}
