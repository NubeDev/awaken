//! Shared org+site scope authorization for the entities that follow the uniform
//! tenancy model — dashboards, flows (boards), and rules. A scope is an `org`
//! plus an optional `site_id` (null = org-level, applying across the org):
//!
//! - **site-scoped** (`site_id = Some`): gate on the owning site's `org/slug`
//!   via [`RequestPrincipal::authorize_site_read`]/`authorize_site_write`.
//! - **org-level** (`site_id = None`): gate on the org scope directly.
//!
//! Reads a caller may not see are filtered before the wire; writes require a
//! write-capable role whose scope covers the target.

use uuid::Uuid;

use crate::auth::{RequestPrincipal, Scope};
use crate::error::ApiError;
use crate::store::{Permission, Store, SubjectKind};

/// Whether the principal may read a resource at `(org, site_id)`. Used both to
/// gate a single get and to filter a list pre-wire.
pub(crate) fn may_read_scope(
    principal: &RequestPrincipal,
    store: &Store,
    org: &str,
    site_id: Option<Uuid>,
) -> bool {
    match site_id {
        Some(site_id) => match store.get_site(site_id) {
            Ok(site) => principal.authorize_site_read(&site.org, &site.slug).is_ok(),
            Err(_) => false,
        },
        None => principal.authorize_read(&Scope::org(org)).is_ok(),
    }
}

/// Authorize a write at `(org, site_id)` — create/patch/delete. Requires a
/// write-capable role whose scope covers the target.
pub(crate) fn authorize_scope_write(
    principal: &RequestPrincipal,
    store: &Store,
    org: &str,
    site_id: Option<Uuid>,
) -> Result<(), ApiError> {
    match site_id {
        Some(site_id) => {
            let site = store.get_site(site_id)?;
            principal.authorize_site_write(&site.org, &site.slug)?;
        }
        None => principal.authorize_write(&Scope::org(org))?,
    }
    Ok(())
}

/// The textual `resource_ref` a grant addresses, for a resource of `kind` in an
/// org. `kind` is `dashboard`/`board`/`rule`; `addr` is the kind-specific tail
/// (a dashboard's uuid, or `<org>/<site|->/<slug-or-name>` for board/rule).
pub(crate) fn resource_ref(kind: &str, addr: &str) -> String {
    format!("{kind}:{addr}")
}

/// The Layer-2 subjects a principal matches against grants: its own user
/// (`user:<id>`) plus each team it belongs to (`team:<id>`). Empty for a
/// pure-token principal (no user row), which then relies on Layer 1 alone.
fn grant_subjects(principal: &RequestPrincipal) -> Vec<(SubjectKind, String)> {
    let Some(p) = principal.0.as_ref() else {
        return Vec::new();
    };
    let mut subjects = Vec::new();
    if let Some(uid) = p.user_id {
        subjects.push((SubjectKind::User, uid.to_string()));
    }
    for tid in &p.team_ids {
        subjects.push((SubjectKind::Team, tid.to_string()));
    }
    subjects
}

/// True when any grant the principal holds in `org` confers `required` (or
/// higher) on the resource — matching either the exact `ref` or a `*` wildcard
/// within the org. The Layer-2 half of [`authorize_resource`].
fn grant_allows(
    principal: &RequestPrincipal,
    store: &Store,
    org: &str,
    kind: &str,
    res_ref: &str,
    required: Permission,
) -> bool {
    let subjects = grant_subjects(principal);
    if subjects.is_empty() {
        return false;
    }
    let Ok(grants) = store.grants_for_subjects(org, &subjects) else {
        return false;
    };
    grants.iter().any(|g| {
        g.resource_kind == kind
            && (g.resource_ref == res_ref || g.resource_ref == "*")
            && g.permission.satisfies(required)
    })
}

/// Two-layer read check: Layer-1 scope-role read OR a Layer-2 read-or-higher
/// grant. Used to gate a single get and to filter a list pre-wire (grants ADD
/// access — a member with no scope read still sees granted resources).
pub(crate) fn may_read_resource(
    principal: &RequestPrincipal,
    store: &Store,
    org: &str,
    site_id: Option<Uuid>,
    kind: &str,
    res_ref: &str,
) -> bool {
    may_read_scope(principal, store, org, site_id)
        || grant_allows(principal, store, org, kind, res_ref, Permission::Read)
}

/// The Layer-2 address tail for a board/rule: `<org>/<site_id|->/<slug-or-name>`.
/// `site_id` (a uuid) keeps the ref deterministic without a slug lookup; `-`
/// marks an org-level resource.
pub(crate) fn scoped_addr(org: &str, site_id: Option<Uuid>, name: &str) -> String {
    let site = site_id.map(|s| s.to_string());
    format!("{org}/{}/{name}", site.as_deref().unwrap_or("-"))
}

/// Two-layer read of a board: scope-role read OR a `board:<ref>` (or `board:*`)
/// read grant.
pub(crate) fn may_read_board(
    principal: &RequestPrincipal,
    store: &Store,
    org: &str,
    site_id: Option<Uuid>,
    slug: &str,
) -> bool {
    let res_ref = resource_ref("board", &scoped_addr(org, site_id, slug));
    may_read_resource(principal, store, org, site_id, "board", &res_ref)
}

/// Two-layer write of a board: scope-role write OR a `board:<ref>`/`board:*`
/// write grant. `slug` is `*` for create (no id yet).
pub(crate) fn authorize_board_write(
    principal: &RequestPrincipal,
    store: &Store,
    org: &str,
    site_id: Option<Uuid>,
    slug: &str,
) -> Result<(), ApiError> {
    let res_ref = if slug == "*" {
        "*".to_string()
    } else {
        resource_ref("board", &scoped_addr(org, site_id, slug))
    };
    authorize_resource_write(principal, store, org, site_id, "board", &res_ref)
}

/// Two-layer read of a rule: scope-role read OR a `rule:<ref>`/`rule:*` read grant.
pub(crate) fn may_read_rule(
    principal: &RequestPrincipal,
    store: &Store,
    org: &str,
    site_id: Option<Uuid>,
    name: &str,
) -> bool {
    let res_ref = resource_ref("rule", &scoped_addr(org, site_id, name));
    may_read_resource(principal, store, org, site_id, "rule", &res_ref)
}

/// Two-layer write of a rule: scope-role write OR a `rule:<ref>`/`rule:*` write
/// grant. `name` is `*` for create.
pub(crate) fn authorize_rule_write(
    principal: &RequestPrincipal,
    store: &Store,
    org: &str,
    site_id: Option<Uuid>,
    name: &str,
) -> Result<(), ApiError> {
    let res_ref = if name == "*" {
        "*".to_string()
    } else {
        resource_ref("rule", &scoped_addr(org, site_id, name))
    };
    authorize_resource_write(principal, store, org, site_id, "rule", &res_ref)
}

/// Two-layer write check: Layer-1 scope-role write OR a Layer-2 write-or-higher
/// grant. The grant path lets a member with no org write still mutate a
/// resource they were granted write on.
pub(crate) fn authorize_resource_write(
    principal: &RequestPrincipal,
    store: &Store,
    org: &str,
    site_id: Option<Uuid>,
    kind: &str,
    res_ref: &str,
) -> Result<(), ApiError> {
    if authorize_scope_write(principal, store, org, site_id).is_ok() {
        return Ok(());
    }
    if grant_allows(principal, store, org, kind, res_ref, Permission::Write) {
        return Ok(());
    }
    // Re-run the scope write to surface its precise Forbidden/NotFound error
    // (the grant path adds access but never a better error than Layer 1's).
    authorize_scope_write(principal, store, org, site_id)
}
