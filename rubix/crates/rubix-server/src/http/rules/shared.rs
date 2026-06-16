//! Shared rule reads and the gate-error mapping the rule routes lean on.
//!
//! A rule is a `kind:"rule"` record, so reads run on the WS-03 scoped session and
//! mutations cross the WS-05 gate like any record. These helpers centralise the
//! two things every rule route needs: load a rule by its *name* (the handle the
//! UI and composition use, distinct from the storage id the gate addresses) on the
//! scoped session, and map a gate failure to its transport status.

use rubix_gate::{ScopedSession, read_records_on_session_filtered};

use crate::dto::rule::{RULE_KIND, RuleDto};
use crate::error::{ApiError, ApiResult};

/// Load every rule visible to `session`, projected to its DTO.
///
/// Reads the `kind:"rule"` collection on the scoped session (row-level
/// permissions decide the rows, contract #1) and drops any record whose content
/// is not a well-formed rule document — a foreign write under the same kind never
/// crashes the listing.
pub(crate) async fn read_rules(session: &ScopedSession) -> ApiResult<Vec<RuleDto>> {
    let records = read_records_on_session_filtered(session, Some(RULE_KIND), &[])
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(records
        .into_iter()
        .filter_map(RuleDto::from_record)
        .collect())
}

/// Load the single rule named `name` on `session`, or `404` if none is visible.
///
/// Names are unique per namespace (the create path enforces it with a `409`), so
/// at most one match exists within the principal's scope.
pub(crate) async fn read_rule_by_name(session: &ScopedSession, name: &str) -> ApiResult<RuleDto> {
    read_rules(session)
        .await?
        .into_iter()
        .find(|rule| rule.name == name)
        .ok_or(ApiError::NotFound)
}

/// Map a gate failure to its transport status: a denied grant is `403`, a
/// validation failure `422`, anything else internal (the write path does not
/// `404` — ids are resolved before the command).
pub(crate) fn map_gate_error(error: rubix_gate::GateError) -> ApiError {
    match error {
        rubix_gate::GateError::CommandDenied(reason) => ApiError::Forbidden(reason),
        rubix_gate::GateError::Validation(reason) => ApiError::Unprocessable(reason),
        other => ApiError::Internal(other.to_string()),
    }
}

/// Evict the writing principal's namespace from the scanned-context cache (§4a).
///
/// A rule write is a data-change the cache must honour so a board reflects the new
/// definition on its next tick — the same invalidation every record mutation does.
pub(crate) fn invalidate_scanned_context(
    state: &crate::state::AppState,
    principal: &rubix_core::Principal,
) {
    state
        .context_cache
        .invalidate_namespace(&principal.namespace);
}
