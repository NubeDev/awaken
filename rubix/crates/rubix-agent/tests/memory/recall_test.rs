//! Integration: the memory seam keeps writes on the gate and recall on the scoped
//! session — the design's safety thesis, end to end.
//!
//! A granted agent persists memories through the gate (an `agent-memory-write`
//! command: authorized, captured, correlated, audited) and then recalls them on
//! its **scoped session**, where SurrealDB row-perms — not a capability — bound
//! what it sees. The nearer embedding ranks first, proving the euclidean search
//! over L2-normalized vectors serves as semantic recall. A principal lacking the
//! grant is refused before any write, proving the write path is real gated code,
//! not an assumed data-plane path.

#[path = "../support/mod.rs"]
mod support;

use rubix_ext::{GrantProfile, grant_extension, register_extension};
use rubix_gate::{PrincipalToken, authenticate, issue_scoped_session};

use rubix_agent::{
    AgentTier, MemoryKind, MemoryRecord, persist_memory, provision_agent, recall_memory,
};
use support::open::{NS, admin, open_agent_store};

const DB: &str = "agent_memory_recall";

#[tokio::test]
async fn a_granted_agent_persists_memory_and_recalls_the_nearest() {
    let handle = open_agent_store(DB).await;

    // An analyst already holds agent-memory-write (recording recall is a mutation
    // even for a read-only agent).
    let agent = provision_agent(handle.raw(), &admin(), "ana", NS, "s3cret", AgentTier::Analyst)
        .await
        .expect("provision analyst agent");

    // Persist two memories with distinct directions through the gate.
    let near = MemoryRecord::new(MemoryKind::Semantic, "chiller plant load", &[1.0, 0.05])
        .expect("near memory");
    let far = MemoryRecord::new(MemoryKind::Semantic, "lobby lighting schedule", &[0.0, 1.0])
        .expect("far memory");

    let persisted_near = persist_memory(handle.raw(), agent.principal(), &near, None)
        .await
        .expect("persist near memory");
    persist_memory(handle.raw(), agent.principal(), &far, None)
        .await
        .expect("persist far memory");

    // The gate carried a correlation id onto the write — provenance is real.
    assert!(!persisted_near.correlation_id.as_str().is_empty());

    // Recall on the agent's own scoped session: row-perms, not a capability.
    let token = PrincipalToken::new("ana", "s3cret");
    let resolved = authenticate(handle.raw(), &token)
        .await
        .expect("authenticate agent");
    let session = issue_scoped_session(handle.raw(), NS, DB, resolved, &token)
        .await
        .expect("issue scoped session");

    // Probe aligned with the "near" memory's direction.
    let hits = recall_memory(&session, &[1.0, 0.0], 2)
        .await
        .expect("recall memory");

    assert_eq!(hits.len(), 2, "both memories are in the agent's namespace");
    // The nearer direction ranks first (smaller euclidean distance).
    assert!(
        hits[0].distance <= hits[1].distance,
        "recall must order nearest-first"
    );
    assert!(
        hits[0].id.contains(persisted_near.memory_id.as_str()),
        "the aligned memory must be the nearest hit: got {}",
        hits[0].id
    );
}

#[tokio::test]
async fn a_principal_without_the_grant_cannot_persist_memory() {
    let handle = open_agent_store("agent_memory_denied").await;

    // A read-only extension: registered, but granted no cross-plane capability —
    // crucially not agent-memory-write.
    let registration = register_extension(handle.raw(), "reader", NS, "s3cret")
        .await
        .expect("register extension");
    let principal = registration.principal().clone();
    grant_extension(handle.raw(), &admin(), &principal, GrantProfile::ReadOnly)
        .await
        .expect("grant read-only profile");

    let memory = MemoryRecord::new(MemoryKind::Working, "should never persist", &[1.0])
        .expect("memory");
    let err = persist_memory(handle.raw(), &principal, &memory, None)
        .await
        .expect_err("a principal without agent-memory-write must be refused");

    assert!(
        matches!(err, rubix_agent::AgentError::MemoryWrite(_)),
        "the gate must deny the write, fail closed: {err:?}"
    );
}
