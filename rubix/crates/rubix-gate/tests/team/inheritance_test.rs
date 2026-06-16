//! Integration: a capability granted to a team is exercisable by its members.
//!
//! The end-to-end proof of team inheritance (`rubix/docs/SCOPE.md`, "Capabilities
//! are grants"): an admin grants a capability to a team, and
//! `check_capability`/`effective_grants` resolve it for every member — while a
//! non-member is still denied, and revoking the team grant (or removing the
//! member) takes the capability away.

#[path = "../gate/mod.rs"]
mod gate;

use rubix_core::{Id, Principal, PrincipalKind, Role};
use rubix_gate::{
    Capability, Team, add_member, check_capability, create_team, create_team_grant,
    effective_grants, remove_member, revoke_team_grant,
};

use gate::open::{NS, open_gate_store};

fn admin() -> Principal {
    Principal::new(Id::from_raw("root"), NS, PrincipalKind::User, Role::Admin)
}

fn member(subject: &str) -> Principal {
    Principal::new(
        Id::from_raw(subject),
        NS,
        PrincipalKind::User,
        Role::Operator,
    )
}

#[tokio::test]
async fn a_team_grant_is_inherited_by_members_only() {
    let handle = open_gate_store("team_inherit").await;
    let admin = admin();
    let alice = member("rubix_alice");
    let bob = member("rubix_bob");

    create_team(handle.raw(), &admin, &Team::new("analysts", NS, "Analysts"))
        .await
        .expect("create team");
    add_member(handle.raw(), &admin, NS, "analysts", "rubix_alice")
        .await
        .expect("add alice");

    // Grant the capability to the TEAM, not to any principal directly.
    create_team_grant(
        handle.raw(),
        &admin,
        "analysts",
        NS,
        Capability::ExternalQuery,
    )
    .await
    .expect("grant team");

    // Alice (a member) inherits it; Bob (not a member) does not.
    assert!(
        check_capability(handle.raw(), &alice, Capability::ExternalQuery)
            .await
            .expect("check alice"),
        "a member must inherit the team grant"
    );
    assert!(
        !check_capability(handle.raw(), &bob, Capability::ExternalQuery)
            .await
            .expect("check bob"),
        "a non-member must not inherit the team grant"
    );

    // The inherited grant shows up in alice's effective set.
    let effective = effective_grants(handle.raw(), &alice)
        .await
        .expect("effective");
    assert!(
        effective
            .iter()
            .any(|g| g.capability == Capability::ExternalQuery),
        "effective grants must include the inherited team grant"
    );
}

#[tokio::test]
async fn revoking_the_team_grant_removes_inherited_access() {
    let handle = open_gate_store("team_revoke").await;
    let admin = admin();
    let alice = member("rubix_alice");

    create_team(handle.raw(), &admin, &Team::new("analysts", NS, "Analysts"))
        .await
        .expect("create team");
    add_member(handle.raw(), &admin, NS, "analysts", "rubix_alice")
        .await
        .expect("add alice");
    create_team_grant(
        handle.raw(),
        &admin,
        "analysts",
        NS,
        Capability::ExternalQuery,
    )
    .await
    .expect("grant team");

    assert!(
        check_capability(handle.raw(), &alice, Capability::ExternalQuery)
            .await
            .expect("check")
    );

    revoke_team_grant(
        handle.raw(),
        &admin,
        "analysts",
        NS,
        Capability::ExternalQuery,
    )
    .await
    .expect("revoke team grant");
    assert!(
        !check_capability(handle.raw(), &alice, Capability::ExternalQuery)
            .await
            .expect("check after revoke"),
        "revoking the team grant must remove inherited access"
    );
}

#[tokio::test]
async fn removing_a_member_removes_inherited_access() {
    let handle = open_gate_store("team_remove_member").await;
    let admin = admin();
    let alice = member("rubix_alice");

    create_team(handle.raw(), &admin, &Team::new("analysts", NS, "Analysts"))
        .await
        .expect("create team");
    add_member(handle.raw(), &admin, NS, "analysts", "rubix_alice")
        .await
        .expect("add alice");
    create_team_grant(
        handle.raw(),
        &admin,
        "analysts",
        NS,
        Capability::ExternalQuery,
    )
    .await
    .expect("grant team");

    assert!(
        check_capability(handle.raw(), &alice, Capability::ExternalQuery)
            .await
            .expect("check")
    );

    remove_member(handle.raw(), &admin, NS, "analysts", "rubix_alice")
        .await
        .expect("remove member");
    assert!(
        !check_capability(handle.raw(), &alice, Capability::ExternalQuery)
            .await
            .expect("check after removal"),
        "a removed member must lose the inherited grant"
    );
}
