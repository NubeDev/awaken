//! The authenticated caller attached to a request after the auth middleware
//! runs. Carries the subject identity, the RBAC [`Scope`] it is confined to, and
//! a coarse role. Domain routes read it from request extensions to gate access.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::scope::Scope;

/// A caller's role. Coarser than the scope: `Operator` is an authenticated human
/// (full read/write within scope), `Service` is a non-interactive account (a
/// driver, the embedded agent's own surface), `Viewer` is read-only. The
/// per-route gate combines this with the [`Scope`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    /// Read-write within scope; an interactive human operator.
    Operator,
    /// Non-interactive machine account (PAT/service account, driver, agent).
    Service,
    /// Read-only within scope.
    Viewer,
}

impl Role {
    /// Parse the canonical token; unknown roles fail closed (`None`).
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "operator" => Some(Role::Operator),
            "service" => Some(Role::Service),
            "viewer" => Some(Role::Viewer),
            _ => None,
        }
    }

    /// The canonical lowercase token.
    pub fn as_str(self) -> &'static str {
        match self {
            Role::Operator => "operator",
            Role::Service => "service",
            Role::Viewer => "viewer",
        }
    }

    /// True when the role may mutate state (operators and service accounts).
    /// Viewers are read-only; a write route rejects them regardless of scope.
    pub fn can_write(self) -> bool {
        matches!(self, Role::Operator | Role::Service)
    }
}

/// The authenticated caller. Produced by the auth verifier from either an OIDC
/// JWT or a PAT, attached to the request, and read by the RBAC gate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Principal {
    /// Stable subject identity (the JWT `sub`, or the PAT id).
    pub subject: String,
    /// The org/team/site the caller is confined to.
    pub scope: Scope,
    /// The caller's coarse role.
    pub role: Role,
}

impl Principal {
    /// Authorize a read against `target`: the principal's scope must cover the
    /// target. Any role may read within scope.
    pub fn may_read(&self, target: &Scope) -> bool {
        self.scope.covers(target)
    }

    /// Authorize a write against `target`: the principal must both be allowed to
    /// write (not a viewer) and have a scope that covers the target.
    pub fn may_write(&self, target: &Scope) -> bool {
        self.role.can_write() && self.scope.covers(target)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_round_trips_and_gates_writes() {
        for role in [Role::Operator, Role::Service, Role::Viewer] {
            assert_eq!(Role::parse(role.as_str()), Some(role));
        }
        assert!(Role::parse("root").is_none());
        assert!(Role::Operator.can_write());
        assert!(Role::Service.can_write());
        assert!(!Role::Viewer.can_write());
    }

    #[test]
    fn viewer_reads_but_cannot_write_in_scope() {
        let p = Principal {
            subject: "u1".into(),
            scope: Scope::org("nube"),
            role: Role::Viewer,
        };
        let target = Scope::org("nube");
        assert!(p.may_read(&target));
        assert!(!p.may_write(&target));
    }

    #[test]
    fn operator_cannot_reach_a_sibling_org() {
        let p = Principal {
            subject: "u1".into(),
            scope: Scope::org("nube"),
            role: Role::Operator,
        };
        assert!(!p.may_read(&Scope::org("acme")));
        assert!(!p.may_write(&Scope::org("acme")));
    }
}
