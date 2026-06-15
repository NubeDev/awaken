//! Attach capability grants to an extension (WS-04).
//!
//! What an extension may *do* is the second authz layer: a set of WS-04
//! capability grants on its principal (`rubix/docs/sessions/WS-13.md`, contract
//! #2). The *same* grant mechanism expresses every extension shape — a read-only
//! extension, an ingest-only extension, and an admin extension differ **only** in
//! which capabilities they are granted, never in a separate trust path. A
//! [`GrantProfile`] names a distinct permitted-action set; [`grant_extension`]
//! confers each capability through the WS-04 [`create_grant`](rubix_gate::create_grant)
//! path, which is itself authority-checked (only a namespace admin may confer a
//! grant — fail closed, no self-escalation).

use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use rubix_core::Principal;
use rubix_gate::{Capability, Grant, create_grant};

use crate::error::{ExtError, Result};

/// A named bundle of capabilities expressing one extension shape.
///
/// The three profiles resolve to **distinct** permitted-action sets from the one
/// grant mechanism (`rubix/docs/sessions/WS-13.md`, "Tests"): only the
/// capabilities differ, not the enforcement path. The order within each set is
/// declaration order; duplicates cannot arise because each capability appears at
/// most once.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrantProfile {
    /// A read-only extension: no cross-plane action capability at all. Its data
    /// reads are still scoped by SurrealDB row-perms (WS-03); it simply holds no
    /// grant to invoke, ingest, or register anything.
    ReadOnly,
    /// An ingest-only extension: may publish into the ingest plane and subscribe
    /// to its Zenoh key-space, nothing else.
    IngestOnly,
    /// An admin extension: may exercise every cross-plane capability the platform
    /// knows about.
    Admin,
    /// An **AI agent analyst** (`rubix/docs/design/AGENT.md`, "Analyst vs.
    /// operator"). Read-only "ask your data": its record reads are scoped by
    /// SurrealDB row-perms (no capability needed for those), it may reach the
    /// DataFusion/Postgres plane (`external-query`), and it may persist memory of
    /// what it read through the gate (`agent-memory-write`) — recording recall is a
    /// mutation, so it crosses the gate even for a read-only analyst. It holds no
    /// rule or actuation authority.
    AgentAnalyst,
    /// An **AI agent operator** — the analyst tier plus the authority to record
    /// rule decisions (`rule-invoke`) and write/enable rule definitions and
    /// schedules (`rule-define`). It changes *configuration and insights*, never a
    /// physical point.
    AgentOperator,
    /// An **AI agent actuator** — the operator tier plus `device-actuate`, the
    /// authority to command a registered physical point. This is the demo's full
    /// tier ("Apply pre-cool to L4 West"). Promoting analyst → operator → actuator
    /// is granting one variant set at a time, fail closed at every step.
    AgentActuator,
}

impl GrantProfile {
    /// The capabilities this profile confers — the permitted action set.
    ///
    /// The three profiles return disjoint-enough sets that they are
    /// distinguishable (`rubix/docs/sessions/WS-13.md`): read-only is empty,
    /// ingest-only is the ingest pair, admin is everything. All three are derived
    /// from the one [`Capability`] enum — no profile invents a capability outside
    /// the WS-04 allow-set.
    /// The three agent tiers are deliberately **layered** — each is a superset of
    /// the one below (analyst ⊂ operator ⊂ actuator), so the static slices below
    /// repeat the lower tier's capabilities rather than compose at runtime (the
    /// return type is a `'static` slice). The
    /// [`agent_tiers_are_strictly_layered`](tests) test pins the subset relation so
    /// the slices cannot drift apart.
    #[must_use]
    pub fn capabilities(self) -> &'static [Capability] {
        match self {
            GrantProfile::ReadOnly => &[],
            GrantProfile::IngestOnly => {
                &[Capability::IngestPublish, Capability::ZenohSubscribe]
            }
            GrantProfile::Admin => &Capability::ALL,
            GrantProfile::AgentAnalyst => {
                &[Capability::ExternalQuery, Capability::AgentMemoryWrite]
            }
            GrantProfile::AgentOperator => &[
                Capability::ExternalQuery,
                Capability::AgentMemoryWrite,
                Capability::RuleInvoke,
                Capability::RuleDefine,
            ],
            GrantProfile::AgentActuator => &[
                Capability::ExternalQuery,
                Capability::AgentMemoryWrite,
                Capability::RuleInvoke,
                Capability::RuleDefine,
                Capability::DeviceActuate,
            ],
        }
    }
}

/// Grant `extension` the capabilities of `profile`, authorized by `grantor`.
///
/// Confers each capability in the profile through the WS-04
/// [`create_grant`](rubix_gate::create_grant) path, which checks `grantor`'s
/// authority before each write (only a namespace admin may confer a grant). A
/// [`ReadOnly`](GrantProfile::ReadOnly) profile writes no grants and returns an
/// empty vector — the extension holds no cross-plane capability, fail closed by
/// default. Returns the grants conferred, in profile order.
///
/// # Errors
/// Returns [`ExtError::Grant`] if `grantor` lacks the authority to confer a
/// grant (not an admin in the extension's namespace) or the write fails. On the
/// first failure no further grants are attempted.
pub async fn grant_extension(
    db: &Surreal<Db>,
    grantor: &Principal,
    extension: &Principal,
    profile: GrantProfile,
) -> Result<Vec<Grant>> {
    let mut conferred = Vec::with_capacity(profile.capabilities().len());
    for &capability in profile.capabilities() {
        let grant = create_grant(db, grantor, extension, capability)
            .await
            .map_err(|e| ExtError::Grant(e.to_string()))?;
        conferred.push(grant);
    }
    Ok(conferred)
}

#[cfg(test)]
mod tests {
    use rubix_gate::Capability;

    use super::GrantProfile;

    #[test]
    fn the_three_profiles_resolve_to_distinct_action_sets() {
        let read_only = GrantProfile::ReadOnly.capabilities();
        let ingest_only = GrantProfile::IngestOnly.capabilities();
        let admin = GrantProfile::Admin.capabilities();

        // Read-only confers no cross-plane capability.
        assert!(read_only.is_empty());
        // Ingest-only is exactly the ingest pair.
        assert_eq!(
            ingest_only,
            &[Capability::IngestPublish, Capability::ZenohSubscribe]
        );
        // Admin is every known capability.
        assert_eq!(admin.len(), Capability::ALL.len());

        // The three sets are pairwise distinct.
        assert_ne!(read_only, ingest_only);
        assert_ne!(ingest_only, admin);
        assert_ne!(read_only, admin);
    }

    #[test]
    fn agent_tiers_are_strictly_layered() {
        let analyst = GrantProfile::AgentAnalyst.capabilities();
        let operator = GrantProfile::AgentOperator.capabilities();
        let actuator = GrantProfile::AgentActuator.capabilities();

        let subset = |inner: &[Capability], outer: &[Capability]| {
            inner.iter().all(|capability| outer.contains(capability))
        };

        // analyst ⊂ operator ⊂ actuator — each tier is a superset of the one
        // below, so promoting a principal is granting capabilities, never a rebuild.
        assert!(subset(analyst, operator));
        assert!(subset(operator, actuator));
        // Each promotion adds authority, so the sets strictly grow.
        assert!(operator.len() > analyst.len());
        assert!(actuator.len() > operator.len());
    }

    #[test]
    fn only_the_actuator_can_command_a_device() {
        // Actuation is fail closed below the top tier: pre-cooling a floor and
        // recording an insight must be distinct grants (AGENT.md, "Actuator").
        assert!(
            !GrantProfile::AgentAnalyst
                .capabilities()
                .contains(&Capability::DeviceActuate)
        );
        assert!(
            !GrantProfile::AgentOperator
                .capabilities()
                .contains(&Capability::DeviceActuate)
        );
        assert!(
            GrantProfile::AgentActuator
                .capabilities()
                .contains(&Capability::DeviceActuate)
        );
    }

    #[test]
    fn the_analyst_records_memory_but_holds_no_rule_authority() {
        let analyst = GrantProfile::AgentAnalyst.capabilities();
        // Recording recall crosses the gate even for a read-only analyst.
        assert!(analyst.contains(&Capability::AgentMemoryWrite));
        // But it cannot record rule decisions or define rules.
        assert!(!analyst.contains(&Capability::RuleInvoke));
        assert!(!analyst.contains(&Capability::RuleDefine));
    }

    #[test]
    fn ingest_only_excludes_admin_only_capabilities() {
        let ingest_only = GrantProfile::IngestOnly.capabilities();
        // An ingest-only extension cannot register a datasource or invoke a rule.
        assert!(!ingest_only.contains(&Capability::DatasourceRegister));
        assert!(!ingest_only.contains(&Capability::RuleInvoke));
        assert!(!ingest_only.contains(&Capability::ExternalQuery));
    }
}
