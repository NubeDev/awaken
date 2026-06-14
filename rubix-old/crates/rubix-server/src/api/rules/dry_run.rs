//! POST /api/v1/orgs/{org}/rules/dry-run — run a rule once against a point's
//! recent history without emitting a spark.
//!
//! This is the debugger spine: the engine already returns a [`RuleResult`]
//! without side effects ([`run_rule`]); this exposes it over HTTP. The input
//! frame is resolved the same way the board `rule` node resolves it — a point
//! keyexpr's history, folded into the canonical two-column (`ts`, `value`)
//! frame via [`rubix_flow::frame_from_his`] — so a dry-run sees the identical
//! frame a live board would. The resolved frame is returned alongside the
//! verdict so the UI can chart what the rule saw.
//!
//! `rubix-rules` stays standalone: its `Severity` shares the canonical
//! lowercase wire string, so [`RuleResult`] serializes directly without a
//! `rubix-core` dependency leaking into the engine. Mapping stays at this
//! boundary, as the rest of the rules integration does.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::Json;
use rubix_core::HisSample;
use rubix_flow::frame_from_his;
use rubix_rules::{
    params_from_json, run_rule, RuleError, RuleResult, RuleSource, RuleStore, SandboxLimits,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

use crate::api::blocking::blocking;
use crate::auth::{RequestPrincipal, Scope};
use crate::error::{ApiError, ErrorBody};
use crate::flow::TableRuleStore;
use crate::store::Store;
use crate::AppState;

/// Default rows resolved from a point's history when the caller gives no limit.
/// A dry-run folds a bounded recent window, not an unbounded read.
const DEFAULT_LIMIT: usize = 500;

/// Hard cap on the dry-run input window, matching the board node's
/// [`rubix_flow::node`] truncation discipline: a rule must not fold an
/// unbounded read.
const MAX_LIMIT: usize = 10_000;

/// Run a rule once and report the verdict plus the frame it saw.
///
/// Exactly one of `script` (inline) or `rule` (stored rule id-or-name) is the
/// source. `point` selects the input window by keyexpr; omit it to dry-run
/// against an empty frame (a compile/shape check). `params` is the JSON map the
/// script reads as `params`.
#[derive(Debug, Deserialize, ToSchema)]
pub struct DryRunRequest {
    /// Inline Rhai script. Mutually exclusive with `rule`.
    #[serde(default)]
    pub script: Option<String>,
    /// Stored rule id-or-name to run. Mutually exclusive with `script`.
    #[serde(default)]
    pub rule: Option<String>,
    /// Parameter map exposed to the script as `params`.
    #[serde(default)]
    pub params: Option<Value>,
    /// Point keyexpr (`{org}/{site}/{equip}/{point}`) whose recent history is
    /// the input frame. Omitted runs against an empty frame.
    #[serde(default)]
    pub point: Option<String>,
    /// Max history rows to resolve (default [`DEFAULT_LIMIT`], capped at
    /// [`MAX_LIMIT`]).
    #[serde(default)]
    pub limit: Option<usize>,
}

/// A single resolved input row, returned so the UI can chart the frame.
#[derive(Debug, Serialize, ToSchema)]
pub struct FrameRow {
    pub ts: String,
    pub value: Option<f64>,
}

/// The resolved input frame summary: row count plus the rows the rule saw.
#[derive(Debug, Serialize, ToSchema)]
pub struct FrameSummary {
    pub row_count: usize,
    pub rows: Vec<FrameRow>,
}

/// The dry-run outcome: the rule's verdict and the frame it ran over.
#[derive(Debug, Serialize, ToSchema)]
pub struct DryRunResponse {
    /// `flagged` / `severity` / `message` / `value` — the rule's decision.
    #[schema(value_type = Object)]
    pub result: RuleResult,
    /// The input frame the rule saw (so the UI can chart it).
    pub frame: FrameSummary,
}

#[utoipa::path(post, path = "/api/v1/orgs/{org}/rules/dry-run", request_body = DryRunRequest,
    tag = "rules", params(("org" = String, Path, description = "Tenant org")),
    security(("bearer" = [])),
    responses((status = 200, body = DryRunResponse), (status = 400, body = ErrorBody),
              (status = 401, body = ErrorBody), (status = 403, body = ErrorBody),
              (status = 404, body = ErrorBody)))]
pub(crate) async fn dry_run_rule(
    State(state): State<AppState>,
    Path(org): Path<String>,
    principal: RequestPrincipal,
    Json(req): Json<DryRunRequest>,
) -> Result<Json<DryRunResponse>, ApiError> {
    principal.authorize_read(&Scope::org(&org))?;

    // Exactly one source, mirroring the board node's `source_config`.
    let source = match (&req.script, &req.rule) {
        (Some(s), None) if !s.trim().is_empty() => Either::Inline(s.clone()),
        (None, Some(r)) if !r.trim().is_empty() => Either::Stored(r.clone()),
        (Some(_), Some(_)) => {
            return Err(ApiError::BadRequest(
                "set exactly one of `script` or `rule`, not both".into(),
            ));
        }
        _ => {
            return Err(ApiError::BadRequest(
                "missing `script` (inline) or `rule` (stored id)".into(),
            ));
        }
    };

    let limit = req.limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT);
    let params = params_from_json(&req.params.clone().unwrap_or(Value::Null));
    let store = state.store.clone();
    let point = req.point.clone();

    // The engine is synchronous and drives DataFusion on its own runtime; run it
    // on a blocking thread (no ambient runtime there, so the engine's `block_on`
    // is free to run — unlike the in-worker board path which needs
    // `block_in_place`).
    blocking(move || {
        let samples = resolve_samples(&store, point.as_deref(), limit)?;
        let frame = frame_from_his(&samples).map_err(ApiError::BadRequest)?;
        // A dry-run previews against org-level composition (no site context).
        let rule_store: Arc<dyn RuleStore> =
            Arc::new(TableRuleStore::new(store.clone(), org.clone(), None));
        let limits = SandboxLimits::default();
        let result = match &source {
            Either::Inline(script) => {
                run_rule(rule_store, RuleSource::Inline(script), frame, params, limits)
            }
            Either::Stored(id) => {
                run_rule(rule_store, RuleSource::Stored(id), frame, params, limits)
            }
        }
        .map_err(rule_error_to_api)?;
        Ok(Json(DryRunResponse {
            result,
            frame: frame_summary(&samples),
        }))
    })
    .await
}

enum Either {
    Inline(String),
    Stored(String),
}

/// Resolve the input rows: a point keyexpr's recent history, or empty when no
/// point is given (a compile/shape dry-run).
fn resolve_samples(
    store: &Store,
    point: Option<&str>,
    limit: usize,
) -> Result<Vec<HisSample>, ApiError> {
    let Some(keyexpr) = point else {
        return Ok(Vec::new());
    };
    let id = store.point_by_keyexpr(keyexpr)?;
    Ok(store.his_query(id, None, None, limit)?)
}

/// The resolved-frame summary the UI charts.
fn frame_summary(samples: &[HisSample]) -> FrameSummary {
    FrameSummary {
        row_count: samples.len(),
        rows: samples
            .iter()
            .map(|s| FrameRow {
                ts: s.ts.to_rfc3339(),
                value: s.value.as_f64(),
            })
            .collect(),
    }
}

/// Map a [`RuleError`] to an HTTP error. A bad rule (compile/runtime/limit) or a
/// composition resolve failure is the caller's input, so it is a 4xx with the
/// category-tagged message the UI surfaces; an engine compute fault is a 500.
fn rule_error_to_api(err: RuleError) -> ApiError {
    match err {
        RuleError::Compile(_)
        | RuleError::Runtime(_)
        | RuleError::LimitExceeded(_)
        | RuleError::Resolve(_) => ApiError::BadRequest(err.to_string()),
        other => ApiError::Internal(anyhow::anyhow!(other)),
    }
}
