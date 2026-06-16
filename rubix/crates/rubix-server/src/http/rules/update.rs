//! `PATCH /rules/:name` — replace a rule's definition through the gate.
//!
//! An update is a mutation, so it crosses the WS-05 gate: the
//! [`RuleDefine`](rubix_gate::Capability::RuleDefine) grant is checked, before/after
//! captured, correlation id minted, write applied, audit row appended. The name is
//! immutable (it is the composition handle other rules reference), so the body
//! carries only the new definition; the handler resolves the rule's storage id by
//! name on the scoped session, validates the draft, then drives the command on
//! that id. The new content keeps the original name.

use axum::Json;
use axum::extract::{Path, State};
use rubix_core::{Id, read_record};
use rubix_gate::{Change, Command, apply};

use crate::auth::Authenticated;
use crate::dto::rule::{RULE_KIND, RuleDoc, RuleDto, UpdateRuleRequest};
use crate::error::{ApiError, ApiResult};
use crate::http::rules::capability::RULE_WRITE;
use crate::http::rules::shared::{invalidate_scanned_context, map_gate_error, read_rule_by_name};
use crate::http::rules::validate::validate_definition;
use crate::state::AppState;

/// Replace the definition of the rule named `name`, through the gate.
pub async fn update_rule_route(
    State(state): State<AppState>,
    auth: Authenticated,
    Path(name): Path<String>,
    Json(body): Json<UpdateRuleRequest>,
) -> ApiResult<Json<RuleDto>> {
    validate_definition(&body.script, &body.inputs)?;

    // Resolve the storage id by name on the session (404 if not visible); the
    // command addresses the record by that id, the name is carried through.
    let existing = read_rule_by_name(&auth.session, &name).await?;
    let id = Id::from_raw(existing.id);

    let doc = RuleDoc {
        kind: RULE_KIND.to_owned(),
        name,
        script: body.script,
        inputs: body.inputs,
        subrules: body.subrules,
        output: body.output,
    };
    let content = serde_json::to_value(&doc).map_err(|e| ApiError::Internal(e.to_string()))?;

    let command = Command::new(
        auth.principal.clone(),
        RULE_WRITE,
        id.clone(),
        Change::Update(content),
    );
    apply(state.store.raw(), &command, None)
        .await
        .map_err(map_gate_error)?;
    invalidate_scanned_context(&state, &auth.principal);

    let stored = read_record(state.store.raw(), &id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound)?;
    RuleDto::from_record(stored)
        .map(Json)
        .ok_or_else(|| ApiError::Internal("stored rule is not a well-formed rule document".into()))
}
