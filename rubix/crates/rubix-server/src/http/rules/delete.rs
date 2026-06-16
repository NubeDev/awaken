//! `DELETE /rules/:name` — delete a rule through the gate.
//!
//! A delete is a mutation, so it crosses the WS-05 gate: the
//! [`RuleDefine`](rubix_gate::Capability::RuleDefine) grant is checked, the
//! before-image captured, correlation id minted, write applied, audit row
//! appended. The handler resolves the rule's storage id by name on the scoped
//! session (404 if not visible), then drives the command and returns `204`.
//!
//! Deleting a rule that other rules compose breaks them on the next tick; the
//! blast radius is surfaced ahead of time by `GET /rules/:name/referencing`, so
//! the delete itself is unconditional.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use rubix_core::Id;
use rubix_gate::{Change, Command, apply};

use crate::auth::Authenticated;
use crate::error::ApiResult;
use crate::http::rules::capability::RULE_WRITE;
use crate::http::rules::shared::{invalidate_scanned_context, map_gate_error, read_rule_by_name};
use crate::state::AppState;

/// Delete the rule named `name` through the gate, returning `204 No Content`.
pub async fn delete_rule_route(
    State(state): State<AppState>,
    auth: Authenticated,
    Path(name): Path<String>,
) -> ApiResult<StatusCode> {
    let existing = read_rule_by_name(&auth.session, &name).await?;
    let command = Command::new(
        auth.principal.clone(),
        RULE_WRITE,
        Id::from_raw(existing.id),
        Change::Delete,
    );
    apply(state.store.raw(), &command, None)
        .await
        .map_err(map_gate_error)?;
    invalidate_scanned_context(&state, &auth.principal);
    Ok(StatusCode::NO_CONTENT)
}
