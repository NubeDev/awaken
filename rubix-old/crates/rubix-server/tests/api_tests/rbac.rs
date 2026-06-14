//! RBAC increments B–E integration: real users + teams + memberships, the
//! two-layer additive authorization (scope-role OR per-resource grant), and the
//! admin tiers (super-admin / org-admin) gating the management surfaces. Driven
//! over the enforced (cloud) PAT path; the synthetic-dev case lives in
//! `whoami.rs`. See `docs/design/authz-rbac.md`.

use axum::http::StatusCode;
use chrono::Utc;
use rubix_core::Dashboard;
use rubix_server::auth::{pat, AdminLevel, Role, Scope, TokenRecord};
use rubix_server::store::{GrantRecord, Permission, Store, SubjectKind, TeamRecord, UserRecord};
use uuid::Uuid;

use super::harness::TestApp;

/// Mint a PAT whose subject resolves to a freshly-seeded user with `admin_level`
/// and `role`/`scope`, optionally in `teams`. Returns `(bearer, user_id)`.
fn seed_user_pat(
    store: &Store,
    org: &str,
    role: Role,
    scope: Scope,
    admin_level: AdminLevel,
    teams: &[Uuid],
) -> (String, Uuid) {
    let minted = pat::mint();
    let user_id = Uuid::new_v4();
    store
        .create_token(&TokenRecord {
            id: minted.id.clone(),
            secret_hash: minted.secret_hash,
            name: "seed".into(),
            role,
            scope,
            created_at: Utc::now(),
            revoked_at: None,
        })
        .expect("seed token");
    store
        .create_user(&UserRecord {
            id: user_id,
            org: org.into(),
            // The PAT id is the verified subject; verify resolves it to this row.
            subject: minted.id.clone(),
            email: format!("{}@{org}.test", minted.id),
            display_name: "seed user".into(),
            admin_level,
            created_at: Utc::now(),
        })
        .expect("seed user");
    for team in teams {
        store.add_team_member(*team, user_id).expect("add member");
    }
    (minted.plaintext, user_id)
}

fn seed_team(store: &Store, org: &str, slug: &str) -> Uuid {
    let id = Uuid::new_v4();
    store
        .create_team(&TeamRecord {
            id,
            org: org.into(),
            slug: slug.into(),
            name: slug.into(),
            created_at: Utc::now(),
        })
        .expect("seed team");
    id
}

fn seed_dashboard(store: &Store, org: &str, slug: &str) -> Uuid {
    let id = Uuid::new_v4();
    store
        .create_dashboard(&Dashboard {
            id,
            org: org.into(),
            site_id: None,
            slug: slug.into(),
            title: slug.into(),
            variables: Vec::new(),
            created_at: Utc::now(),
        })
        .expect("seed dashboard");
    id
}

fn seed_grant(
    store: &Store,
    org: &str,
    kind: SubjectKind,
    subject_id: Uuid,
    dashboard_id: Uuid,
    perm: Permission,
) {
    store
        .create_grant(&GrantRecord {
            id: Uuid::new_v4(),
            org: org.into(),
            subject_kind: kind,
            subject_id: subject_id.to_string(),
            resource_kind: "dashboard".into(),
            resource_ref: format!("dashboard:{dashboard_id}"),
            permission: perm,
            created_at: Utc::now(),
        })
        .expect("seed grant");
}

/// The headline scenario: a team member with **no org write** reads dashboard A
/// (team read grant), writes dashboard B (team write grant), but is 403/404 on
/// dashboard C (no grant). Grants ADD access on top of a read-only scope.
#[tokio::test]
async fn team_grants_add_per_dashboard_access() {
    let (app, store) = TestApp::with_auth();
    let team = seed_team(&store, "acme", "ops");
    // Viewer scope: read-only at org level, no write anywhere.
    let (bearer, _uid) = seed_user_pat(
        &store,
        "acme",
        Role::Viewer,
        Scope::org("acme"),
        AdminLevel::None,
        &[team],
    );
    let dash_a = seed_dashboard(&store, "acme", "a");
    let dash_b = seed_dashboard(&store, "acme", "b");
    let dash_c = seed_dashboard(&store, "acme", "c");
    // Team gets read on A, write on B; nothing on C.
    seed_grant(&store, "acme", SubjectKind::Team, team, dash_a, Permission::Read);
    seed_grant(&store, "acme", SubjectKind::Team, team, dash_b, Permission::Write);

    // Reads: A and B both visible (B's write grant satisfies read); C visible too
    // via the org-level Viewer scope read (Layer 1). The discriminating check is
    // writes below.
    for id in [dash_a, dash_b, dash_c] {
        let (status, _) = app
            .request_as("GET", &format!("/api/v1/dashboards/{id}"), &bearer, None)
            .await;
        assert_eq!(status, StatusCode::OK, "viewer reads {id} via scope");
    }

    // Write B: allowed by the team write grant despite no org write.
    let (status, body) = app
        .request_as(
            "PATCH",
            &format!("/api/v1/dashboards/{dash_b}"),
            &bearer,
            Some(serde_json::json!({"title": "B edited"})),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "write B via grant: {body}");

    // Write A: only a read grant → forbidden.
    let (status, _) = app
        .request_as(
            "PATCH",
            &format!("/api/v1/dashboards/{dash_a}"),
            &bearer,
            Some(serde_json::json!({"title": "A edited"})),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "write A: read grant only");

    // Write C: no grant at all → forbidden.
    let (status, _) = app
        .request_as(
            "PATCH",
            &format!("/api/v1/dashboards/{dash_c}"),
            &bearer,
            Some(serde_json::json!({"title": "C edited"})),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "write C: no grant");
}

/// A direct (`user:<id>`) write grant works the same as a team grant, and a
/// member with NO org read at all still sees a granted dashboard (grants add
/// read access where scope gives none).
#[tokio::test]
async fn direct_grant_adds_read_for_otherwise_blind_member() {
    let (app, store) = TestApp::with_auth();
    // A user scoped to a *different* org has no Layer-1 read on acme at all.
    let (bearer, uid) = seed_user_pat(
        &store,
        "acme",
        Role::Viewer,
        Scope::org("other"),
        AdminLevel::None,
        &[],
    );
    let dash = seed_dashboard(&store, "acme", "secret");
    // Without a grant: not visible (404, the filter hides it).
    let (status, _) = app
        .request_as("GET", &format!("/api/v1/dashboards/{dash}"), &bearer, None)
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "no scope, no grant: hidden");
    // Grant the user read directly.
    seed_grant(&store, "acme", SubjectKind::User, uid, dash, Permission::Read);
    let (status, _) = app
        .request_as("GET", &format!("/api/v1/dashboards/{dash}"), &bearer, None)
        .await;
    assert_eq!(status, StatusCode::OK, "direct read grant reveals it");
}

/// An org-admin manages its org's teams and grants; a plain operator cannot.
#[tokio::test]
async fn org_admin_manages_teams_and_grants() {
    let (app, store) = TestApp::with_auth();
    let (admin, _) = seed_user_pat(
        &store,
        "acme",
        Role::Operator,
        Scope::org("acme"),
        AdminLevel::OrgAdmin,
        &[],
    );
    // Create a team via the API (admin-gated).
    let (status, team) = app
        .request_as(
            "POST",
            "/api/v1/orgs/acme/teams",
            &admin,
            Some(serde_json::json!({"slug": "eng", "name": "Engineering"})),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "org-admin creates team: {team}");

    // A non-admin operator in the same org is refused.
    let (op, _) = seed_user_pat(
        &store,
        "acme",
        Role::Operator,
        Scope::org("acme"),
        AdminLevel::None,
        &[],
    );
    let (status, _) = app
        .request_as(
            "POST",
            "/api/v1/orgs/acme/teams",
            &op,
            Some(serde_json::json!({"slug": "x", "name": "X"})),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "non-admin cannot manage teams");
}

/// An org-admin of acme cannot manage another org, but a super-admin crosses
/// every org.
#[tokio::test]
async fn admin_scope_confinement_and_super_admin_crossing() {
    let (app, store) = TestApp::with_auth();
    let (org_admin, _) = seed_user_pat(
        &store,
        "acme",
        Role::Operator,
        Scope::org("acme"),
        AdminLevel::OrgAdmin,
        &[],
    );
    // org-admin of acme is refused on org `other`.
    let (status, _) = app
        .request_as("GET", "/api/v1/orgs/other/teams", &org_admin, None)
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "org-admin confined to its org");

    // super-admin (global) reaches both orgs.
    let (super_admin, _) = seed_user_pat(
        &store,
        "acme",
        Role::Operator,
        Scope::org("acme"),
        AdminLevel::SuperAdmin,
        &[],
    );
    for org in ["acme", "other"] {
        let (status, _) = app
            .request_as("GET", &format!("/api/v1/orgs/{org}/teams"), &super_admin, None)
            .await;
        assert_eq!(status, StatusCode::OK, "super-admin manages {org}");
    }
}

/// Cross-tenant denial holds: an org-admin of acme cannot read a user that lives
/// in org `other`, even by guessing the path org.
#[tokio::test]
async fn cross_tenant_user_access_denied() {
    let (app, store) = TestApp::with_auth();
    let (acme_admin, _) = seed_user_pat(
        &store,
        "acme",
        Role::Operator,
        Scope::org("acme"),
        AdminLevel::OrgAdmin,
        &[],
    );
    // A user in `other`.
    let other_user = Uuid::new_v4();
    store
        .create_user(&UserRecord {
            id: other_user,
            org: "other".into(),
            subject: "other-sub".into(),
            email: "u@other.test".into(),
            display_name: "other".into(),
            admin_level: AdminLevel::None,
            created_at: Utc::now(),
        })
        .expect("seed other user");
    // acme-admin is refused at the `other` org path entirely.
    let (status, _) = app
        .request_as(
            "GET",
            &format!("/api/v1/orgs/other/users/{other_user}"),
            &acme_admin,
            None,
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "acme-admin cannot read other's user");
}

/// `whoami` reflects the resolved admin principal under the enforced PAT path
/// (the auth-off synthetic case is covered in `whoami.rs`).
#[tokio::test]
async fn whoami_reports_resolved_admin_principal() {
    let (app, store) = TestApp::with_auth();
    let (super_admin, _) = seed_user_pat(
        &store,
        "acme",
        Role::Operator,
        Scope::org("acme"),
        AdminLevel::SuperAdmin,
        &[],
    );
    let (status, body) = app
        .request_as("GET", "/api/v1/whoami", &super_admin, None)
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    // A super-admin resolves to a global Admin role, regardless of token scope.
    assert_eq!(body["role"], "admin");
    assert_eq!(body["auth_enabled"], true);
    assert_eq!(body["can_write"], true);
    // scope is global (org omitted).
    assert!(body["scope"]["org"].is_null(), "super-admin scope is global");
}

/// Management mutations are NOT open when auth is off: with no principal the
/// admin gate denies, a deliberate deviation from the resource-gate no-op. (The
/// default edge harness has auth off.)
#[tokio::test]
async fn management_routes_denied_when_auth_off() {
    let app = TestApp::new();
    // No bearer, auth off: a resource read is still open…
    let (status, _) = app.request("GET", "/api/v1/sites", None).await;
    assert_eq!(status, StatusCode::OK, "resource read open when auth off");
    // …but team management is denied (require_admin needs a real principal).
    let (status, _) = app
        .request(
            "POST",
            "/api/v1/orgs/acme/teams",
            Some(serde_json::json!({"slug": "x", "name": "X"})),
        )
        .await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "management mutation denied when auth off"
    );
}
