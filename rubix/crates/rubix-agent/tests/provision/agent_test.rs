//! Integration: an agent is provisioned as a scoped principal and granted exactly
//! its tier — fail closed below it.
//!
//! Exercises the agent's identity model end to end against a live kv-mem
//! SurrealDB: `provision_agent` registers an `Extension`-kind principal on the
//! same WS-03 path a user/extension uses and confers its [`AgentTier`]'s WS-04
//! grants, authorized by a namespace admin. The granted tier is then checked
//! through the real gate `check_capability` path: an actuator holds
//! `device-actuate`, an analyst does not — the layering is enforced by the gate,
//! not assumed in the type.

#[path = "../support/mod.rs"]
mod support;

use rubix_core::PrincipalKind;
use rubix_gate::{Capability, check_capability};

use rubix_agent::{AgentTier, provision_agent};
use support::open::{NS, admin, open_agent_store};

#[tokio::test]
async fn an_actuator_agent_holds_the_full_layered_tier() {
    let handle = open_agent_store("agent_provision_actuator").await;

    let agent = provision_agent(
        handle.raw(),
        &admin(),
        "avery",
        NS,
        "s3cret",
        AgentTier::Actuator,
    )
    .await
    .expect("provision actuator agent");

    // It is an extension-kind principal bound to the namespace.
    assert_eq!(agent.principal().kind, PrincipalKind::Extension);
    assert_eq!(agent.principal().namespace, NS);
    assert_eq!(agent.tier(), AgentTier::Actuator);

    // The full layered tier is granted, checked through the real gate path.
    for capability in [
        Capability::ExternalQuery,
        Capability::AgentMemoryWrite,
        Capability::RuleInvoke,
        Capability::RuleDefine,
        Capability::DeviceActuate,
    ] {
        let allowed = check_capability(handle.raw(), agent.principal(), capability)
            .await
            .expect("check capability");
        assert!(allowed, "actuator must hold {capability:?}");
    }
}

#[tokio::test]
async fn an_analyst_agent_cannot_actuate_or_define_rules() {
    let handle = open_agent_store("agent_provision_analyst").await;

    let agent = provision_agent(
        handle.raw(),
        &admin(),
        "ana",
        NS,
        "s3cret",
        AgentTier::Analyst,
    )
    .await
    .expect("provision analyst agent");

    // An analyst records memory of what it read and may reach the external plane.
    for granted in [Capability::ExternalQuery, Capability::AgentMemoryWrite] {
        assert!(
            check_capability(handle.raw(), agent.principal(), granted)
                .await
                .expect("check capability"),
            "analyst must hold {granted:?}"
        );
    }

    // But it holds no rule or actuation authority — fail closed below its tier.
    for denied in [
        Capability::RuleInvoke,
        Capability::RuleDefine,
        Capability::DeviceActuate,
    ] {
        assert!(
            !check_capability(handle.raw(), agent.principal(), denied)
                .await
                .expect("check capability"),
            "analyst must NOT hold {denied:?}"
        );
    }
}
