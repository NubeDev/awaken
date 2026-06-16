//! `POST /rules/:name/dryrun` — run a draft rule against real history, no firing.
//!
//! The debugger's tight edit→run loop: author a rule and see what it would decide
//! against the principal's real window history *without* recording an insight,
//! crossing the command gate, or emitting a trace. The draft (which may be
//! unsaved) travels in the body; any stored sub-rules it composes are loaded from
//! the scoped session into the dry-run registry. Resolution runs on the WS-03
//! scoped session, so SurrealDB row-level permissions decide the rows the rule
//! sees (contract #1) — a dry-run can never read outside the principal's scope.
//!
//! This is a read in effect (it computes a verdict and returns it), so it does not
//! gate on [`RuleDefine`](rubix_gate::Capability::RuleDefine): authoring is gated,
//! but trying a draft against one's own visible data is not a mutation. The
//! `rubix-rules` dry-run is side-effect-free by construction.

use axum::Json;
use axum::extract::Path;
use rubix_rules::{RuleError, RuleRegistry, dry_run};

use crate::auth::Authenticated;
use crate::dto::rule::{BucketDto, DryRunRequest, DryRunResponse, ResolvedInputDto, build_rule};
use crate::error::{ApiError, ApiResult};
use crate::http::rules::shared::read_rules;

/// The id the draft rule is registered under for the dry-run — a transient handle
/// that never reaches storage, distinct from any stored rule's name.
const DRAFT_ID: &str = "__draft__";

/// Dry-run the draft in the body against `name`'s history on the scoped session.
///
/// `name` is the route handle for the rule being debugged; the actual script and
/// inputs come from the body so the on-screen (possibly unsaved) draft is what
/// runs. Returns the verdict and the frame each binding resolved from. A compile,
/// binding, or window failure maps to `400` with the engine's message verbatim.
pub async fn dryrun_rule_route(
    auth: Authenticated,
    Path(_name): Path<String>,
    Json(body): Json<DryRunRequest>,
) -> ApiResult<Json<DryRunResponse>> {
    // Build the dry-run registry: the draft under a transient id, plus every
    // stored sub-rule it composes (loaded from the principal's scoped session).
    let mut registry = RuleRegistry::new();
    let draft = build_rule(
        DRAFT_ID,
        &body.script,
        &body.inputs,
        &body.subrules,
        "dry-run",
    )
    .map_err(|reason| ApiError::BadRequest(format!("binding: {reason}")))?;
    registry.insert(draft);

    if !body.subrules.is_empty() {
        let stored = read_rules(&auth.session).await?;
        for sub_name in &body.subrules {
            let sub = stored.iter().find(|r| r.name == *sub_name).ok_or_else(|| {
                ApiError::BadRequest(format!("sub-rule `{sub_name}` is not a stored rule"))
            })?;
            let rule = build_rule(
                &sub.name,
                &sub.script,
                &sub.inputs,
                &sub.subrules,
                &sub.output,
            )
            .map_err(|reason| {
                ApiError::BadRequest(format!("sub-rule `{sub_name}` binding: {reason}"))
            })?;
            registry.insert(rule);
        }
    }

    let result = dry_run(auth.session.connection(), &registry, DRAFT_ID)
        .await
        .map_err(map_rule_error)?;

    let inputs = result
        .inputs
        .into_iter()
        .map(|input| ResolvedInputDto {
            name: input.name,
            value: input.value,
            buckets: input
                .buckets
                .into_iter()
                .map(|b| BucketDto {
                    bucket_start: b.bucket_start,
                    avg: b.avg,
                    min: b.min,
                    max: b.max,
                    sum: b.sum,
                    count: b.count,
                    first: b.first,
                    last: b.last,
                })
                .collect(),
        })
        .collect();

    Ok(Json(DryRunResponse {
        fired: result.decision.fired,
        value: result.decision.value,
        reason: result.decision.reason,
        inputs,
    }))
}

/// Map a dry-run engine failure to its transport status.
///
/// A compile, binding, window, or evaluation failure is the *author's* problem —
/// the draft is wrong or its data is missing — so it is a `400` carrying the
/// engine's diagnostic verbatim (it is engine output, not user markup). An unknown
/// sub-rule reaching here (already pre-checked) and a span/record path (never hit
/// on the dry-run) fall through to a `500`.
fn map_rule_error(error: RuleError) -> ApiError {
    match error {
        RuleError::Compile(m)
        | RuleError::Evaluate(m)
        | RuleError::Binding(m)
        | RuleError::Window(m)
        | RuleError::NotFound(m) => ApiError::BadRequest(m),
        other => ApiError::Internal(other.to_string()),
    }
}
