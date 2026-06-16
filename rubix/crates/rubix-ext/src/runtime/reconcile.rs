//! Boot reconciler: bring the supervisor map into agreement with the records.
//!
//! On host boot nothing is running, but the gated `lifecycle` records persist the
//! desired state across reboots (`rubix/docs/design/EXTENSION-RUNTIME.md`,
//! "boot-time reconciler"). This reads those records and starts every extension
//! last left in `start`, leaving `stop`/`disable` down — the gate-native
//! equivalent of starter's "EnablementStore queried at boot", with no side table.
//! It is idempotent: re-running it never double-spawns a live extension (the
//! supervisor registry's `start` is itself idempotent).
//!
//! An extension's desired state is read from its control record's `content`:
//!
//! - `lifecycle` — the [`LifecycleAction`] string the gate last wrote. A record
//!   without it is not an extension control record and is skipped.
//! - `runtime` — the [`ProcessSpec`] to spawn (required for a `start`).
//! - `extension` — the principal subject the supervisor keys by; falls back to
//!   the record id when absent.
//!
//! The child's credentials (Open question 2) are resolved through a caller-
//! supplied closure rather than read from the record — secrets never live in a
//! readable content field. A `start` whose identity cannot be resolved is skipped
//! with a reason, never spawned credential-less.

use rubix_core::Record;
use rubix_gate::{ScopedSession, read_records_on_session};

use crate::control::LifecycleAction;
use crate::error::{ExtError, Result};
use crate::supervisor::{ExtensionId, Identity, ProcessSpec};

use super::ExtensionRuntime;

/// What the reconciler did, for boot logging and tests.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ReconcileReport {
    /// Extensions that were `start` and are now supervised.
    pub started: Vec<ExtensionId>,
    /// Extensions that were `stop`/`disable` and were torn down (had a live
    /// supervisor that was shut down).
    pub stopped: Vec<ExtensionId>,
    /// Records skipped, paired with the reason (not an extension record, a
    /// `start` with no runtime spec or unresolved identity, a non-process
    /// flavour, or a spawn failure).
    pub skipped: Vec<(ExtensionId, String)>,
}

/// One extension's desired runtime state, parsed from its control record.
struct Desired {
    id: ExtensionId,
    action: LifecycleAction,
    spec: Option<ProcessSpec>,
}

/// Parse a record into a desired runtime state, or `None` if it is not an
/// extension control record (no recognised `lifecycle` field).
fn parse(record: &Record) -> Option<Desired> {
    let content = &record.content;
    let action = content
        .get("lifecycle")
        .and_then(serde_json::Value::as_str)
        .and_then(LifecycleAction::parse)?;
    let subject = content
        .get("extension")
        .and_then(serde_json::Value::as_str)
        .map_or_else(|| record.id.as_str().to_owned(), str::to_owned);
    let id = ExtensionId::new(record.namespace.clone(), subject);
    let spec = content
        .get("runtime")
        .and_then(|r| serde_json::from_value::<ProcessSpec>(r.clone()).ok());
    Some(Desired { id, action, spec })
}

/// Reconcile the supervisor map against a set of already-read control `records`.
///
/// `resolve_identity` supplies a child's credentials by id; returning `None`
/// skips that `start` rather than spawning it credential-less. Pure logic over
/// the provided records — the DB read is [`reconcile_on_session`]'s job — so this
/// is unit-testable without a live engine.
pub async fn reconcile_from_records<F>(
    rt: &ExtensionRuntime,
    records: &[Record],
    resolve_identity: F,
) -> ReconcileReport
where
    F: Fn(&ExtensionId) -> Option<Identity>,
{
    let mut report = ReconcileReport::default();

    for record in records {
        let Some(desired) = parse(record) else {
            continue;
        };
        match desired.action {
            LifecycleAction::Start => {
                let Some(spec) = desired.spec else {
                    report
                        .skipped
                        .push((desired.id, "start with no runtime spec".to_owned()));
                    continue;
                };
                if !spec.flavour.reports_process_stats() {
                    report
                        .skipped
                        .push((desired.id, "non-process flavour: no child to spawn".to_owned()));
                    continue;
                }
                let Some(identity) = resolve_identity(&desired.id) else {
                    report
                        .skipped
                        .push((desired.id, "could not resolve child identity".to_owned()));
                    continue;
                };
                match rt.supervisors.start(desired.id.clone(), spec, identity) {
                    Ok(_) => report.started.push(desired.id),
                    Err(e) => report.skipped.push((desired.id, format!("spawn failed: {e}"))),
                }
            }
            LifecycleAction::Stop | LifecycleAction::Disable => {
                if rt.supervisors.stop(&desired.id).await {
                    report.stopped.push(desired.id);
                }
            }
        }
    }

    report
}

/// Read the control records visible to `session` and reconcile the supervisor map
/// against them.
///
/// The boot path calls this once per namespace session; reads are SurrealDB-
/// native (row-level permissions scope what the session sees), so it only ever
/// reconciles the session's own namespace.
///
/// # Errors
/// Returns [`ExtError::Command`] if the scoped record read fails.
pub async fn reconcile_on_session<F>(
    rt: &ExtensionRuntime,
    session: &ScopedSession,
    resolve_identity: F,
) -> Result<ReconcileReport>
where
    F: Fn(&ExtensionId) -> Option<Identity>,
{
    let records = read_records_on_session(session)
        .await
        .map_err(|e| ExtError::Command(e.to_string()))?;
    Ok(reconcile_from_records(rt, &records, resolve_identity).await)
}
