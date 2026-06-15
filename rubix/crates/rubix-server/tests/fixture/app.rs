//! Boot the transport over an in-memory store with the gate schema applied and a
//! granted principal provisioned — the fixture the HTTP/WS/OpenAPI tests run on.
//!
//! The WS-16 integration tests exercise the real route table on kv-mem
//! (`rubix/docs/sessions/WS-16.md`): mutations must cross the WS-05 gate (with an
//! audit row written) and reads must run on the WS-03 scoped session. This
//! fixture opens the store, declares the gate + audit schema, provisions an
//! operator principal, grants it the record-write capability (the gate's
//! `IngestPublish`, the capability the transport routes a record mutation
//! through), and returns the assembled `Router` plus the principal's credential
//! headers.
//!
//! The fixture is shared across test binaries via `#[path]`, and each binary uses
//! only the helpers it needs (the records tests use [`boot`]; the admin tests use
//! [`boot_admin`]). The unused-across-a-given-binary lint is therefore allowed for
//! the whole module rather than per item.
#![allow(dead_code)]

use axum::Router;
use rubix_core::{Id, Principal, PrincipalKind, Role, RuntimeConfig};
use rubix_gate::{Capability, PrincipalToken, authenticate, create_grant, provision_principal};
use rubix_server::profile::Profile;
use rubix_server::{AppState, router};
use rubix_store::StoreHandle;

/// Namespace every server test runs against.
pub const NS: &str = "rubix";
/// The provisioned principal's subject.
pub const SUBJECT: &str = "operator";
/// The provisioned principal's secret.
pub const SECRET: &str = "pw";

/// A booted transport plus the handle and credential headers a test drives it
/// with.
pub struct TestApp {
    /// The assembled transport router.
    pub app: Router,
    /// The store owner handle, for asserting on persisted rows (e.g. audit).
    ///
    /// Not every test binary reads the handle (the fixture is shared via
    /// `#[path]`), so the unused-field lint is allowed here rather than per test.
    #[allow(dead_code)]
    pub store: StoreHandle,
}

/// Boot the transport with the gate schema applied and a granted principal.
///
/// `capabilities` are granted to the principal by an admin in `NS` (the gate's
/// no-escalation rule), so a test can choose which actions the principal may
/// perform. `database` keeps each test's kv-mem instance isolated.
pub async fn boot(database: &str, capabilities: &[Capability]) -> TestApp {
    let cfg = RuntimeConfig::in_memory(NS, database);
    let store = StoreHandle::open(&cfg).await.expect("open in-memory store");
    rubix_gate::define_gate_schema(store.raw())
        .await
        .expect("define gate schema");
    rubix_gate::define_audit_schema(store.raw())
        .await
        .expect("define audit schema");

    let principal = Principal::new(Id::from_raw(SUBJECT), NS, PrincipalKind::User, Role::Operator);
    provision_principal(store.raw(), &principal, SECRET)
        .await
        .expect("provision principal");

    let admin = Principal::new(Id::from_raw("admin"), NS, PrincipalKind::User, Role::Admin);
    for capability in capabilities {
        create_grant(store.raw(), &admin, &principal, *capability)
            .await
            .expect("grant capability");
    }

    // Confirm the credential resolves before any test relies on it.
    authenticate(store.raw(), &PrincipalToken::new(SUBJECT, SECRET))
        .await
        .expect("authenticate provisioned principal");

    let app = router(AppState::new(store.clone(), NS, database));
    TestApp { app, store }
}

/// The admin principal's **API-local** subject for the admin-surface tests.
///
/// Stored namespace-prefixed (`{NS}_root`) to match the production onboarding
/// convention; the admin authenticates with the full subject but addresses itself
/// over the API by this local form.
pub const ADMIN_SUBJECT: &str = "root";
/// The admin principal's **full** subject — the credential it signs in with.
pub const ADMIN_FULL_SUBJECT: &str = "rubix_root";
/// The admin principal's secret.
pub const ADMIN_SECRET: &str = "root-pw";

/// Boot the transport with an `Admin` principal in `NS` provisioned and granted
/// `capabilities` — the fixture the admin-surface tests drive.
///
/// The admin endpoints require `Role::Admin` in the caller's namespace, so this
/// provisions an admin (unlike [`boot`], which provisions an operator). The admin
/// is itself the `state.namespace` admin, so it also satisfies the root/system
/// rule the tenant routes check on the edge default profile.
pub async fn boot_admin(database: &str, capabilities: &[Capability]) -> TestApp {
    let app_state =
        |store: StoreHandle, db: &str| AppState::new(store, NS, db);
    boot_admin_inner(database, capabilities, app_state).await
}

/// Like [`boot_admin`] but threads a selected deployment [`Profile`] into state —
/// the fixture the cloud-only tenant onboarding test drives.
///
/// The same root admin is provisioned in `NS` (the configured root namespace), so
/// it satisfies the root/system rule the tenant routes check regardless of profile.
pub async fn boot_admin_with_profile(
    database: &str,
    capabilities: &[Capability],
    profile: Profile,
) -> TestApp {
    let app_state =
        move |store: StoreHandle, db: &str| AppState::with_profile(store, NS, db, profile.clone());
    boot_admin_inner(database, capabilities, app_state).await
}

/// Shared setup for the admin fixtures: schema, root admin, grants, then build the
/// router via the caller-supplied `AppState` constructor (which selects the profile).
async fn boot_admin_inner(
    database: &str,
    capabilities: &[Capability],
    build_state: impl FnOnce(StoreHandle, &str) -> AppState,
) -> TestApp {
    let cfg = RuntimeConfig::in_memory(NS, database);
    let store = StoreHandle::open(&cfg).await.expect("open in-memory store");
    rubix_gate::define_gate_schema(store.raw())
        .await
        .expect("define gate schema");
    rubix_gate::define_audit_schema(store.raw())
        .await
        .expect("define audit schema");
    // The admin surface's tenant registry lives in its own config table; define it
    // so the onboarding/listing routes have somewhere to write, as the binary does.
    rubix_server::define_tenant_schema(store.raw())
        .await
        .expect("define tenant schema");

    let admin = Principal::new(
        Id::from_raw(ADMIN_FULL_SUBJECT),
        NS,
        PrincipalKind::User,
        Role::Admin,
    );
    provision_principal(store.raw(), &admin, ADMIN_SECRET)
        .await
        .expect("provision admin");
    for capability in capabilities {
        create_grant(store.raw(), &admin, &admin, *capability)
            .await
            .expect("grant capability");
    }

    let app = router(build_state(store.clone(), database));
    TestApp { app, store }
}
