//! Drive a command through the gate: authorize → validate → correlate → capture → audit.
//!
//! Contract #1 (`rubix/STACK-DEISGN.md`): every mutation crosses the gate, which
//! checks the capability grant, mints/carries the correlation id, captures
//! before/after atomically with the write, applies the change, and writes the
//! audit row. This is the single write-enforcement point — the orchestrator that
//! sequences those steps. The principal is already authenticated (resolved by
//! [`authenticate`](crate::authenticate) before the command is built), so this
//! pipeline begins at the authorization decision and fails closed: a denied
//! command never reaches capture, so no record is written and no audit row is
//! produced.

use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use rubix_core::CorrelationId;

use crate::audit::{AuditRecord, append_audit};
use crate::error::Result;

use super::authorize::authorize;
use super::capture::{CapturedChange, capture};
use super::correlate::correlate;
use super::define::Command;
use super::validate::validate;

/// The result of a command applied through the gate.
///
/// Carries the before/after captured atomically with the write and the
/// correlation id stamped onto the audit row — the thread a caller follows into
/// the trace and undo planes (`rubix/docs/SCOPE.md`, "Correlation id").
#[derive(Debug, Clone, PartialEq)]
pub struct Applied {
    /// The before/after pair captured atomically with the write.
    pub captured: CapturedChange,
    /// The correlation id the command (and its audit row) ran under.
    pub correlation_id: CorrelationId,
}

/// Apply `command` through the gate, carrying `correlation` if supplied.
///
/// Sequences the gate pipeline: check the capability grant (refuse before any
/// write), validate the content against its collection contract, resolve the
/// correlation id, capture before/after atomically with the write, then append
/// the immutable audit row. Returns the captured change and correlation id on
/// success.
///
/// # Errors
/// Returns [`GateError::CommandDenied`](crate::GateError::CommandDenied) if the
/// principal lacks the grant, [`GateError::Validation`](crate::GateError::Validation)
/// if the content fails its collection contract,
/// [`GateError::CommandApply`](crate::GateError::CommandApply) if the mutation
/// fails, or [`GateError::AuditWrite`](crate::GateError::AuditWrite) if the audit
/// append fails.
pub async fn apply(
    db: &Surreal<Db>,
    command: &Command,
    correlation: Option<CorrelationId>,
) -> Result<Applied> {
    authorize(db, command).await?;
    validate(db, command).await?;
    let correlation_id = correlate(correlation);
    let captured = capture(db, command.namespace(), &command.target, &command.change).await?;
    let audit = AuditRecord::project(
        &command.principal,
        command.change.action(),
        &command.target,
        &captured,
        &correlation_id,
    );
    append_audit(db, &audit).await?;
    Ok(Applied {
        captured,
        correlation_id,
    })
}
