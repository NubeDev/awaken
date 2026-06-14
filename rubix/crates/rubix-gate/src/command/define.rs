//! The `Command` — the unit of mutation that crosses the gate.
//!
//! Contract #1 (`rubix/STACK-DEISGN.md`): every mutation crosses the gate, which
//! authenticates the principal, checks its capability grant, captures
//! before/after for audit+undo, mints/carries the correlation id, and only then
//! applies the change. A `Command` bundles everything that decision needs: the
//! principal acting, the capability the action requires (the WS-04 check), the
//! target record id, and the intended [`Change`]. The gate namespace is the
//! principal's own namespace — a command never targets another tenant.

use rubix_core::{Id, Principal};

use crate::capability::Capability;

use super::action::Change;

/// A single mutation submitted to the gate for enforcement and application.
///
/// The command does not itself apply anything; [`apply`](super::apply::apply)
/// drives it through the gate pipeline. The `capability` is the app-enforced
/// grant the principal must hold (`rubix/docs/SCOPE.md`, "Two authz layers");
/// `target` is the record id the [`change`](Command::change) acts on.
#[derive(Debug, Clone)]
pub struct Command {
    /// The principal submitting the command — the subject of authz and audit.
    pub principal: Principal,
    /// The capability grant the principal must hold to apply this command.
    pub capability: Capability,
    /// The record id the change targets.
    pub target: Id,
    /// The intended record mutation.
    pub change: Change,
}

impl Command {
    /// Build a command for `principal` to apply `change` to `target`, gated by
    /// `capability`.
    #[must_use]
    pub fn new(
        principal: Principal,
        capability: Capability,
        target: Id,
        change: Change,
    ) -> Self {
        Self {
            principal,
            capability,
            target,
            change,
        }
    }

    /// The namespace the command operates in — always the principal's own.
    ///
    /// A command cannot target another tenant: the namespace is taken from the
    /// principal, not supplied independently, so there is no cross-namespace
    /// write path (mirrors the row-level read scope of WS-03).
    #[must_use]
    pub fn namespace(&self) -> &str {
        &self.principal.namespace
    }
}

#[cfg(test)]
mod tests {
    use rubix_core::{Id, Principal, PrincipalKind, Role};

    use crate::capability::Capability;

    use super::{Change, Command};

    #[test]
    fn namespace_is_the_principals_own() {
        let principal = Principal::new(
            Id::from_raw("alice"),
            "tenant-a",
            PrincipalKind::User,
            Role::Operator,
        );
        let command = Command::new(
            principal,
            Capability::RuleInvoke,
            Id::from_raw("rec-1"),
            Change::Delete,
        );
        assert_eq!(command.namespace(), "tenant-a");
    }
}
