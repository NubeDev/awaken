//! Integration: team + membership CRUD, `teams_of`, and authority — live engine.
//!
//! Exercises the gate-owned grouping primitive against a real SurrealDB
//! (`rubix/STACK-DEISGN.md`, kv-mem for tests): an admin creates teams and adds
//! members, `teams_of` resolves a principal's teams, deleting a team drops its
//! memberships, and a non-admin (or cross-namespace admin) is refused.

#[path = "../gate/mod.rs"]
mod gate;

use rubix_core::{Id, Principal, PrincipalKind, Role};
use rubix_gate::{
    Team, add_member, create_team, delete_team, get_team, list_members, list_teams, remove_member,
    teams_of,
};

use gate::open::{NS, open_gate_store};

fn admin() -> Principal {
    Principal::new(Id::from_raw("root"), NS, PrincipalKind::User, Role::Admin)
}

fn user(subject: &str) -> Principal {
    Principal::new(Id::from_raw(subject), NS, PrincipalKind::User, Role::Viewer)
}

#[tokio::test]
async fn team_crud_round_trips() {
    let handle = open_gate_store("team_crud").await;
    let admin = admin();

    let team = Team::new("engineers", NS, "Engineers");
    create_team(handle.raw(), &admin, &team)
        .await
        .expect("create");

    let fetched = get_team(handle.raw(), NS, "engineers")
        .await
        .expect("get")
        .expect("present");
    assert_eq!(fetched.display_name, "Engineers");

    let all = list_teams(handle.raw(), NS).await.expect("list");
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].slug, "engineers");

    delete_team(handle.raw(), &admin, NS, "engineers")
        .await
        .expect("delete");
    assert!(
        get_team(handle.raw(), NS, "engineers")
            .await
            .expect("get")
            .is_none(),
        "deleted team must be gone"
    );
}

#[tokio::test]
async fn membership_drives_teams_of() {
    let handle = open_gate_store("team_membership").await;
    let admin = admin();
    let alice = user("rubix_alice");

    create_team(
        handle.raw(),
        &admin,
        &Team::new("engineers", NS, "Engineers"),
    )
    .await
    .expect("create engineers");
    create_team(handle.raw(), &admin, &Team::new("oncall", NS, "On-call"))
        .await
        .expect("create oncall");

    add_member(handle.raw(), &admin, NS, "engineers", "rubix_alice")
        .await
        .expect("add to engineers");
    add_member(handle.raw(), &admin, NS, "oncall", "rubix_alice")
        .await
        .expect("add to oncall");

    let mut slugs = teams_of(handle.raw(), &alice).await.expect("teams_of");
    slugs.sort();
    assert_eq!(slugs, vec!["engineers".to_owned(), "oncall".to_owned()]);

    let members = list_members(handle.raw(), NS, "engineers")
        .await
        .expect("members");
    assert_eq!(members, vec!["rubix_alice".to_owned()]);

    // Removing the membership drops the team from the principal's set.
    remove_member(handle.raw(), &admin, NS, "oncall", "rubix_alice")
        .await
        .expect("remove from oncall");
    let slugs = teams_of(handle.raw(), &alice).await.expect("teams_of");
    assert_eq!(slugs, vec!["engineers".to_owned()]);
}

#[tokio::test]
async fn deleting_a_team_drops_its_memberships() {
    let handle = open_gate_store("team_delete_cascade").await;
    let admin = admin();
    let alice = user("rubix_alice");

    create_team(
        handle.raw(),
        &admin,
        &Team::new("engineers", NS, "Engineers"),
    )
    .await
    .expect("create");
    add_member(handle.raw(), &admin, NS, "engineers", "rubix_alice")
        .await
        .expect("add");

    delete_team(handle.raw(), &admin, NS, "engineers")
        .await
        .expect("delete");

    // The membership is gone, so the principal belongs to no team.
    let slugs = teams_of(handle.raw(), &alice).await.expect("teams_of");
    assert!(slugs.is_empty(), "membership must be dropped with the team");
}

#[tokio::test]
async fn a_non_admin_may_not_administer_teams() {
    let handle = open_gate_store("team_authority").await;
    let operator = Principal::new(Id::from_raw("op"), NS, PrincipalKind::User, Role::Operator);

    let err = create_team(handle.raw(), &operator, &Team::new("x", NS, "X"))
        .await
        .expect_err("non-admin must be refused");
    assert!(err.to_string().contains("may not"), "got: {err}");
}

#[tokio::test]
async fn an_admin_may_not_administer_another_namespaces_team() {
    let handle = open_gate_store("team_cross_ns").await;
    let foreign_admin = Principal::new(
        Id::from_raw("root"),
        "other-tenant",
        PrincipalKind::User,
        Role::Admin,
    );

    let err = create_team(handle.raw(), &foreign_admin, &Team::new("x", NS, "X"))
        .await
        .expect_err("cross-namespace admin must be refused");
    assert!(err.to_string().contains("may not"), "got: {err}");
}
