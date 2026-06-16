//! `POST /rules` — author a rule through the WS-05 command gate.
//!
//! Authoring a rule is a mutation, so it crosses the gate (`rubix/docs/SCOPE.md`,
//! "Commands go through the gate"): the gate checks the principal's
//! [`RuleDefine`](rubix_gate::Capability::RuleDefine) grant, captures before/after,
//! mints the correlation id, applies the write, and appends the audit row. Before
//! the command the handler validates the draft — a lowercase-slug name, a script
//! that compiles, and well-formed binding enums — and refuses a duplicate name
//! with `409`, so a broken or colliding rule never reaches storage. The rule is
//! stored as a `kind:"rule"` record; its content is the rule document.

use axum::Json;
use axum::extract::State;
use rubix_core::{Id, read_record};
use rubix_gate::{Change, Command, apply};

use crate::auth::Authenticated;
use crate::dto::rule::{CreateRuleRequest, RULE_KIND, RuleDoc, RuleDto};
use crate::error::{ApiError, ApiResult};
use crate::http::rules::capability::RULE_WRITE;
use crate::http::rules::shared::{invalidate_scanned_context, map_gate_error, read_rules};
use crate::http::rules::validate::{validate_definition, validate_name};
use crate::state::AppState;

/// Create a rule carrying the request definition, attributed to the principal.
///
/// Fails with `400` on an invalid name, script, or binding; `409` if the name is
/// already taken in the principal's namespace; `403` if the grant is missing.
pub async fn create_rule_route(
    State(state): State<AppState>,
    auth: Authenticated,
    Json(body): Json<CreateRuleRequest>,
) -> ApiResult<Json<RuleDto>> {
    validate_name(&body.name)?;
    validate_definition(&body.script, &body.inputs)?;

    // A rule's name is its composition handle, so it must be unique within the
    // namespace — reject a collision before the write rather than silently
    // shadowing the existing rule.
    if read_rules(&auth.session)
        .await?
        .iter()
        .any(|rule| rule.name == body.name)
    {
        return Err(ApiError::Conflict(format!(
            "a rule named `{}` already exists in this namespace",
            body.name
        )));
    }

    let doc = RuleDoc {
        kind: RULE_KIND.to_owned(),
        name: body.name,
        script: body.script,
        inputs: body.inputs,
        subrules: body.subrules,
        output: body.output,
    };
    let content = serde_json::to_value(&doc).map_err(|e| ApiError::Internal(e.to_string()))?;

    let id = Id::new();
    let command = Command::new(
        auth.principal.clone(),
        RULE_WRITE,
        id.clone(),
        Change::Create(content),
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
