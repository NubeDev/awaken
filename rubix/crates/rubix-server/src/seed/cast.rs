//! Provision the demo principal cast for a tenant under `--seed-dev`.
//!
//! Identity provisioning has no HTTP path (it precedes any scoped session), so
//! the dev seed does it through the gate library directly — the same calls the
//! WS-16 test fixture uses (`provision_principal` + `create_grant`). Each tenant
//! gets the four-principal cast that exercises both authz layers: an operator
//! that may write (the `IngestPublish` grant a record mutation routes through)
//! and run the query console (`ExternalQuery`), a read-only viewer, an analyst
//! that may run DataFusion queries (`ExternalQuery`), and an extension service
//! account for the agent design (`ExternalQuery` + `RuleInvoke`).

use rubix_core::{Id, Principal, PrincipalKind, Role};
use rubix_gate::{
    Capability, Team, add_member, create_grant, create_team, create_team_grant, provision_principal,
};
use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use super::SeedError;

/// One demo principal template: its credential, identity, and granted caps.
struct DemoPrincipal {
    /// The credential subject (kept short; namespaced per tenant at provision).
    subject: &'static str,
    /// The shared secret the principal signs in with — a demo credential.
    secret: &'static str,
    /// Whether the principal is a user or an extension service account.
    kind: PrincipalKind,
    /// The principal's coarse authority band.
    role: Role,
    /// The capabilities an admin grants the principal.
    grants: &'static [Capability],
}

/// The per-tenant cast, covering every role / kind / capability combination the
/// transport can exercise today.
const CAST: &[DemoPrincipal] = &[
    DemoPrincipal {
        subject: "admin",
        secret: "admin-demo",
        kind: PrincipalKind::User,
        role: Role::Admin,
        grants: &[Capability::IngestPublish, Capability::DatasourceRegister],
    },
    DemoPrincipal {
        subject: "operator",
        secret: "operator-demo",
        kind: PrincipalKind::User,
        role: Role::Operator,
        grants: &[
            Capability::IngestPublish,
            Capability::DatasourceRegister,
            // The query console is an operator-facing tool; without this the
            // default operator login 403s on POST /query (external-query gate).
            Capability::ExternalQuery,
            // Authoring rules in the studio is an operator action; the seed also
            // writes the demo rules as the operator, so it needs this grant
            // (`/rules` mutations gate on RuleDefine, distinct from RuleInvoke).
            Capability::RuleDefine,
            // The seed plays the poller and bulk-appends time-series history via
            // `POST /readings`, which gates on `readings-append` (the data-plane
            // write, distinct from the `ingest-publish` Zenoh stream).
            Capability::ReadingsAppend,
            // Uploading a file's bytes (`POST /files`) is its own fail-closed grant,
            // distinct from the record write that later stores the reference.
            Capability::FileUpload,
        ],
    },
    DemoPrincipal {
        subject: "viewer",
        secret: "viewer-demo",
        kind: PrincipalKind::User,
        role: Role::Viewer,
        grants: &[],
    },
    DemoPrincipal {
        subject: "analyst",
        secret: "analyst-demo",
        kind: PrincipalKind::User,
        role: Role::Operator,
        grants: &[Capability::ExternalQuery],
    },
    DemoPrincipal {
        subject: "agent",
        secret: "agent-demo",
        kind: PrincipalKind::Extension,
        role: Role::Operator,
        grants: &[Capability::ExternalQuery, Capability::RuleInvoke],
    },
];

/// One demo team: its slug/label plus the member subjects (bare, namespaced at
/// seed time) and the capabilities granted to the team (inherited by members).
struct DemoTeam {
    /// The team's slug within the tenant.
    slug: &'static str,
    /// The team's human label.
    display_name: &'static str,
    /// The bare subjects of the cast members to add (namespaced per tenant).
    members: &'static [&'static str],
    /// Capabilities granted to the team — every member inherits these.
    grants: &'static [Capability],
}

/// The per-tenant demo teams. `engineers` groups the operator + analyst and is
/// granted the query console capability **at the team level**, demonstrating an
/// inherited grant: a member exercises `ExternalQuery` through membership, not a
/// per-principal grant. This is the shape the UI's team-access view opens onto
/// (`rubix/docs/SCOPE.md`, teams).
const TEAMS: &[DemoTeam] = &[
    DemoTeam {
        slug: "engineers",
        display_name: "Engineers",
        members: &["operator", "analyst"],
        grants: &[Capability::ExternalQuery],
    },
    DemoTeam {
        slug: "viewers",
        display_name: "Viewers",
        members: &["viewer"],
        grants: &[],
    },
];

/// A provisioned credential, returned so the seed can print a login table.
pub struct Credential {
    /// The full subject the client authenticates with (`{namespace}-{role}`).
    pub subject: String,
    /// The secret paired with the subject.
    pub secret: &'static str,
    /// The capabilities the principal holds.
    pub grants: &'static [Capability],
}

/// Provision the full cast for `namespace`, returning the operator principal
/// (which writes the portfolio) plus every credential for the login summary.
///
/// Subjects are namespaced (`acme_operator`) because the `principal` table is
/// keyed by subject across the whole store, so two tenants cannot share the bare
/// `operator` key. The separator is an underscore, not a hyphen: the access
/// method's `SIGNIN` builds the record id by string concatenation
/// (`type::record('principal:' + $subject)`), which does not parse a hyphenated
/// key. Grants are conferred by an in-memory admin in the same namespace — the
/// grantor authority the gate's no-escalation rule checks.
pub async fn provision_cast(
    db: &Surreal<Db>,
    namespace: &str,
) -> Result<(Principal, Vec<Credential>), SeedError> {
    let admin = Principal::new(
        Id::from_raw(format!("{namespace}_admin")),
        namespace.to_owned(),
        PrincipalKind::User,
        Role::Admin,
    );

    let mut operator = None;
    let mut credentials = Vec::with_capacity(CAST.len());

    for member in CAST {
        let subject = format!("{namespace}_{}", member.subject);
        let principal = Principal::new(
            Id::from_raw(subject.clone()),
            namespace.to_owned(),
            member.kind,
            member.role,
        );
        provision_principal(db, &principal, member.secret)
            .await
            .map_err(|e| SeedError::new("provision principal", e))?;

        for capability in member.grants {
            create_grant(db, &admin, &principal, *capability)
                .await
                .map_err(|e| SeedError::new("grant capability", e))?;
        }

        if member.subject == "operator" {
            operator = Some(principal);
        }
        credentials.push(Credential {
            subject,
            secret: member.secret,
            grants: member.grants,
        });
    }

    // Provision the demo teams: create each, add its members, and grant it its
    // team-level capabilities — so a member inherits access through membership.
    for team in TEAMS {
        create_team(
            db,
            &admin,
            &Team::new(team.slug, namespace.to_owned(), team.display_name),
        )
        .await
        .map_err(|e| SeedError::new("create team", e))?;
        for member in team.members {
            let subject = format!("{namespace}_{member}");
            add_member(db, &admin, namespace, team.slug, &subject)
                .await
                .map_err(|e| SeedError::new("add team member", e))?;
        }
        for capability in team.grants {
            create_team_grant(db, &admin, team.slug, namespace, *capability)
                .await
                .map_err(|e| SeedError::new("grant team capability", e))?;
        }
    }

    let operator = operator.expect("cast always contains an operator");
    Ok((operator, credentials))
}
