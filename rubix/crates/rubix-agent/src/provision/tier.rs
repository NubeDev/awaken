//! The agent's authority tier — the layered grant set it is provisioned with.
//!
//! `rubix/docs/design/AGENT.md` ("Analyst vs. operator", "Actuator") defines the
//! agent as a scoped principal whose reach is decided by **row perms first, one
//! capability tier second** — not a bag of invented grants. The three tiers are
//! strictly layered (analyst ⊂ operator ⊂ actuator); promoting an agent is
//! granting the next tier, never a rebuild. This type is the agent-facing name for
//! that decision; it resolves to the [`GrantProfile`](rubix_ext::GrantProfile)
//! the WS-04 grant path already enforces, so the agent reuses the one grant
//! mechanism rather than defining a parallel trust path.

use rubix_ext::GrantProfile;

/// The capability tier an agent principal is provisioned at.
///
/// Each tier is a strict superset of the one below (the layering is pinned by the
/// `GrantProfile` tests in `rubix-ext`). The default-est, safest tier is
/// [`Analyst`](AgentTier::Analyst): read-only "ask your data", with memory recall
/// recorded through the gate. There is no tier below it that can still record
/// memory, because recording recall is itself a gated mutation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentTier {
    /// Read-only analyst: scoped reads, `external-query` for the DataFusion/
    /// Postgres plane, and `agent-memory-write` to record what it read. No rule or
    /// actuation authority.
    Analyst,
    /// Operator: analyst plus recording rule decisions (`rule-invoke`) and writing
    /// rule definitions/schedules (`rule-define`). Changes configuration and
    /// insights, never a physical point.
    Operator,
    /// Actuator: operator plus `device-actuate` — the authority to command a
    /// registered physical point. The demo's full tier.
    Actuator,
}

impl AgentTier {
    /// The [`GrantProfile`] this tier provisions, resolving the agent tier onto
    /// the WS-04 grant set `rubix-ext` already enforces.
    #[must_use]
    pub fn profile(self) -> GrantProfile {
        match self {
            AgentTier::Analyst => GrantProfile::AgentAnalyst,
            AgentTier::Operator => GrantProfile::AgentOperator,
            AgentTier::Actuator => GrantProfile::AgentActuator,
        }
    }
}

#[cfg(test)]
mod tests {
    use rubix_ext::GrantProfile;

    use super::AgentTier;

    #[test]
    fn each_tier_resolves_to_its_grant_profile() {
        assert_eq!(AgentTier::Analyst.profile(), GrantProfile::AgentAnalyst);
        assert_eq!(AgentTier::Operator.profile(), GrantProfile::AgentOperator);
        assert_eq!(AgentTier::Actuator.profile(), GrantProfile::AgentActuator);
    }
}
