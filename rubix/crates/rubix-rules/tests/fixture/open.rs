//! An in-memory store, a granted principal, and seeded readings — the fixture
//! the rule-evaluation tests run on.
//!
//! A rule fires offline on real window values, records its insight through the
//! gate, publishes the firing, and traces the evaluation
//! (`rubix/docs/sessions/WS-11.md`). This fixture opens a kv-mem store with the
//! gate + audit + trace schema applied, provisions a principal and grants it the
//! `rule-invoke` capability, issues its scoped session, and seeds a numeric
//! reading series — everything one evaluation needs, no cloud dependency
//! (`rubix/STACK-DEISGN.md`, "Key decisions": kv-mem for tests).

use rubix_core::{Id, Principal, PrincipalKind, Record, Role, RuntimeConfig, create_record};
use rubix_gate::{
    Capability, PrincipalToken, ScopedSession, authenticate, create_grant, issue_scoped_session,
    provision_principal,
};
use rubix_store::StoreHandle;
use surrealdb::types::Datetime;

/// Namespace every rule test runs against.
pub const NS: &str = "rubix";

/// Open an in-memory store with the gate, audit, and trace schema applied.
pub async fn open_rules_store(database: &str) -> StoreHandle {
    let cfg = RuntimeConfig::in_memory(NS, database);
    let handle = StoreHandle::open(&cfg).await.expect("open in-memory store");
    rubix_gate::define_gate_schema(handle.raw())
        .await
        .expect("define gate schema");
    rubix_gate::define_audit_schema(handle.raw())
        .await
        .expect("define audit schema");
    rubix_trace::define_trace_schema(handle.raw())
        .await
        .expect("define trace schema");
    handle
}

/// Provision `subject` as an operator in `NS`, grant it `rule-invoke`, and issue
/// its scoped session.
///
/// The grant is conferred by an admin in the same namespace (the gate's
/// no-escalation rule), so the returned principal may record an insight through
/// the gate.
pub async fn granted_session(
    handle: &StoreHandle,
    database: &str,
    subject: &str,
) -> (Principal, ScopedSession) {
    let principal = Principal::new(
        Id::from_raw(subject),
        NS,
        PrincipalKind::User,
        Role::Operator,
    );
    provision_principal(handle.raw(), &principal, "pw")
        .await
        .expect("provision principal");

    let admin = Principal::new(Id::from_raw("admin"), NS, PrincipalKind::User, Role::Admin);
    create_grant(handle.raw(), &admin, &principal, Capability::RuleInvoke)
        .await
        .expect("grant rule-invoke");

    let token = PrincipalToken::new(subject, "pw");
    let resolved = authenticate(handle.raw(), &token)
        .await
        .expect("authenticate");
    let session = issue_scoped_session(handle.raw(), NS, database, resolved.clone(), &token)
        .await
        .expect("issue scoped session");
    (resolved, session)
}

/// Provision `subject` as an operator in `NS` with **no** grant, and issue its
/// scoped session — used to prove the gate denies an ungranted evaluation.
///
/// Only the `record` test binary exercises the denial path; the fixture is
/// compiled into every test binary via `#[path]`, so the others see this as
/// unused. The allow is scoped to this one helper, not the whole module.
#[allow(dead_code)]
pub async fn ungranted_session(
    handle: &StoreHandle,
    database: &str,
    subject: &str,
) -> (Principal, ScopedSession) {
    let principal = Principal::new(
        Id::from_raw(subject),
        NS,
        PrincipalKind::User,
        Role::Operator,
    );
    provision_principal(handle.raw(), &principal, "pw")
        .await
        .expect("provision principal");
    let token = PrincipalToken::new(subject, "pw");
    let resolved = authenticate(handle.raw(), &token)
        .await
        .expect("authenticate");
    let session = issue_scoped_session(handle.raw(), NS, database, resolved.clone(), &token)
        .await
        .expect("issue scoped session");
    (resolved, session)
}

/// Seed a reading in `NS` with a chosen `created` instant and numeric `field`
/// value — one sample of the window series a rule rolls up.
pub async fn seed_reading(handle: &StoreHandle, field: &str, at_secs: i64, value: f64) {
    let mut record = Record::new(NS, serde_json::json!({ field: value }));
    record.id = Id::new();
    record.created = Datetime::from_timestamp(at_secs, 0).expect("valid instant");
    record.updated = record.created;
    create_record(handle.raw(), &record)
        .await
        .expect("seed reading");
}
