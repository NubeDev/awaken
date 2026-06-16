//! Shared helpers for the `/extensions*` admin surface.
//!
//! The surface is per-namespace by construction (`rubix/docs/design/ADMIN-API.md`):
//! every read runs on the caller's WS-03 scoped session, so SurrealDB row-level
//! permissions confine it to the caller's namespace, and the supervisor/metrics
//! gauges are keyed by an [`ExtensionId`] built from that same namespace. A tenant
//! admin therefore only ever sees and drives its own extensions — there is no
//! cross-namespace path here, not even by id.
//!
//! An extension's durable state lives in its **control record** (the one
//! `rubix-ext` `register`/`configure`/`lifecycle` write): `content.runtime` is the
//! [`ProcessSpec`], `content.lifecycle` the last gated action, and
//! `content.extension` the principal subject the runtime keys by. These helpers
//! resolve that record and fold the live in-memory gauges over it.

use rubix_core::Record;
use rubix_ext::metrics::ProcessGauges;
use rubix_ext::supervisor::{ExtensionId, LifecycleState, ProcessFlavour, ProcessSpec};
use rubix_gate::{ScopedSession, read_records_on_session};

use crate::auth::Authenticated;
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

/// The extension id the runtime keys by, scoped to the caller's namespace.
pub(super) fn ext_id(auth: &Authenticated, subject: &str) -> ExtensionId {
    ExtensionId::new(auth.principal.namespace.clone(), subject)
}

/// The principal subject an extension control record belongs to: its explicit
/// `content.extension`, or the record id when absent.
pub(super) fn subject_of(record: &Record) -> String {
    record
        .content
        .get("extension")
        .and_then(serde_json::Value::as_str)
        .map_or_else(|| record.id.as_str().to_owned(), str::to_owned)
}

/// Whether a record is an extension control record (carries a `lifecycle` or
/// `runtime` field). Other records in the namespace are ignored by this surface.
pub(super) fn is_control_record(record: &Record) -> bool {
    let c = &record.content;
    c.get("lifecycle").is_some() || c.get("runtime").is_some()
}

/// Read every extension control record visible to the caller's session.
///
/// # Errors
/// Returns [`ApiError::Internal`] if the scoped read fails.
pub(super) async fn read_control_records(session: &ScopedSession) -> ApiResult<Vec<Record>> {
    let records = read_records_on_session(session)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(records.into_iter().filter(is_control_record).collect())
}

/// Find the control record for `subject` on the caller's session.
///
/// # Errors
/// Returns [`ApiError::Internal`] if the scoped read fails.
pub(super) async fn find_control_record(
    session: &ScopedSession,
    subject: &str,
) -> ApiResult<Option<Record>> {
    let records = read_control_records(session).await?;
    Ok(records.into_iter().find(|r| subject_of(r) == subject))
}

/// Parse the [`ProcessSpec`] off a control record's `content.runtime`, if present
/// and well-formed.
pub(super) fn parse_spec(record: &Record) -> Option<ProcessSpec> {
    record
        .content
        .get("runtime")
        .and_then(|r| serde_json::from_value::<ProcessSpec>(r.clone()).ok())
}

/// The packaging flavour declared on a control record, defaulting to `Process`.
pub(super) fn flavour_of(record: &Record) -> ProcessFlavour {
    parse_spec(record).map_or(ProcessFlavour::Process, |s| s.flavour)
}

/// Build the process gauges for an extension, from the live supervisor handle
/// when one is registered, or degraded to `fallback_state` + zeros otherwise
/// (builtin/wasm, stopped, or never started). Mirrors starter's graceful
/// degradation so every known extension yields a meaningful metrics document.
pub(super) fn gauges_for(
    state: &AppState,
    id: &ExtensionId,
    fallback_state: LifecycleState,
) -> ProcessGauges {
    match state.extensions.supervisors.get(id) {
        Some(h) => ProcessGauges {
            process: h.process_stats(),
            lifecycle_state: h.lifecycle_state(),
            restarts_total: h.restarts_total(),
            capability_violations_total: h.capability_violations(),
            events_dropped_total: h.events_dropped(),
        },
        None => ProcessGauges {
            process: None,
            lifecycle_state: fallback_state,
            restarts_total: 0,
            capability_violations_total: 0,
            events_dropped_total: 0,
        },
    }
}
