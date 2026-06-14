//! An in-memory store and a principal granted (or not) `zenoh-subscribe` — the
//! fixture the ingest integration tests run on.
//!
//! Ingestion scopes the Zenoh key-space once at subscribe via the WS-04 gate and
//! persists append-only, edge-partitioned records (`rubix/docs/sessions/WS-12.md`).
//! This fixture opens a kv-mem store with the gate + audit schema applied,
//! provisions a principal, optionally grants it the `zenoh-subscribe` capability
//! (conferred by an admin in the same namespace, the gate's no-escalation rule),
//! and exposes the namespace every test partitions on — everything one ingest run
//! needs, no cloud dependency (`rubix/STACK-DEISGN.md`, kv-mem for tests).

use rubix_core::{Id, Principal, PrincipalKind, Role, RuntimeConfig};
use rubix_gate::{Capability, create_grant, provision_principal};
use rubix_store::StoreHandle;

/// Namespace (edge identity) every ingest test partitions on.
pub const NS: &str = "edge-7";

/// Open an in-memory store with the gate and audit schema applied.
pub async fn open_ingest_store(database: &str) -> StoreHandle {
    let cfg = RuntimeConfig::in_memory(NS, database);
    let handle = StoreHandle::open(&cfg).await.expect("open in-memory store");
    rubix_gate::define_gate_schema(handle.raw())
        .await
        .expect("define gate schema");
    rubix_gate::define_audit_schema(handle.raw())
        .await
        .expect("define audit schema");
    handle
}

/// Provision `subject` as an operator in `NS` and grant it `zenoh-subscribe`.
///
/// The grant is conferred by an admin in the same namespace, so the returned
/// principal may open an ingest subscription on its own edge key-space.
pub async fn granted_principal(handle: &StoreHandle, subject: &str) -> Principal {
    let principal = provisioned(handle, subject).await;
    let admin = Principal::new(Id::from_raw("admin"), NS, PrincipalKind::User, Role::Admin);
    create_grant(handle.raw(), &admin, &principal, Capability::ZenohSubscribe)
        .await
        .expect("grant zenoh-subscribe");
    principal
}

/// Provision `subject` as an operator in `NS` with **no** grant — used to prove
/// the gate refuses an ungranted subscribe.
///
/// Only the `authorize` test binary exercises the denial path; the fixture is
/// compiled into every test binary via `#[path]`, so the others see this as
/// unused. The allow is scoped to this one helper, not the whole module.
#[allow(dead_code)]
pub async fn ungranted_principal(handle: &StoreHandle, subject: &str) -> Principal {
    provisioned(handle, subject).await
}

async fn provisioned(handle: &StoreHandle, subject: &str) -> Principal {
    let principal = Principal::new(Id::from_raw(subject), NS, PrincipalKind::User, Role::Operator);
    provision_principal(handle.raw(), &principal, "pw")
        .await
        .expect("provision principal");
    principal
}
