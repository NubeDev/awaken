# Authorization & RBAC — Users, Teams, Admin Tiers, and Per-Resource Grants

Scope for turning the lightweight scope+role gate into a real authorization
system: **users** and **teams** as first-class entities, three **admin tiers**
(super-admin / org-admin / member), and **per-resource grants** so a team can be
given read or write on specific dashboards, flows, and rules — the Grafana /
Niagara model of "select exactly what a user or team can see and edit."

This layers on the existing tenancy (`org` + optional `site_id`, see
[crud-and-tenancy.md](crud-and-tenancy.md)); it does not change where the tenant
boundary is drawn, only who within a tenant may do what.

## Problem

Today (verified across `crates/rubix-server/src/auth/`):

1. **No users or teams.** Identity is just a token subject string (a PAT id or
   an OIDC `sub`). There is no `users`/`teams`/`memberships` table, so "add a
   user to a team and give the team access to dashboards A, B, C" is unexpressible.
2. **Role is coarse and binary.** `Role` is `Operator | Service | Viewer`;
   authorization is `can_write()` (write-capable or read-only) **× scope cover**.
   There is no per-resource permission — a principal that can write in an org can
   write *every* dashboard/flow/rule in it. You cannot grant write on dashboard A
   but read on dashboard B.
3. **`team` is vestigial.** `Scope` carries a `team: Option<String>`, persisted in
   `tokens.scope_team` and JWT claims, but `Scope::covers_resource` **ignores it**
   ([scope.rs](../../crates/rubix-server/src/auth/scope.rs)). It grants nothing.
4. **No admin surface and no `whoami`.** The UI holds an opaque bearer token with
   zero role/scope awareness ([auth-store.ts](../../ui/src/stores/auth-store.ts)),
   so it can't show or hide admin pages, or render a user/team management surface.
5. **Auth is off in dev** (edge profile, `auth_required: false`). Every gate is a
   no-op until OIDC is configured. The model below must hold *with auth on* and
   stay a clean no-op with it off (the existing `RequestPrincipal(None)` convention).

## The model (best-long-term)

Two layers, evaluated in order. A request is allowed if **either** grants it.

### Layer 1 — scope-role (the cheap common case)

A principal has a **scope** (`org` + optional `site_id`) and a **role**. This is
today's model, kept: most access is "this team edits everything in site Y." The
role becomes a small ladder (below). Layer-1 alone covers the 90% case without a
single grant row.

### Layer 2 — per-resource grants (the precise override)

A **grant** row pins a permission on a specific resource (or a wildcard) for a
**subject** (a user or a team):

```
grant(subject, resource, permission)
  subject    = user:<id> | team:<id>
  resource   = dashboard:<id> | board:<org/site?/slug> | rule:<org/site?/name> | *  (within a scope)
  permission = read | write | admin
```

A grant **adds** access (it never subtracts — deny-by-omission, like Grafana).
The resolved decision for `(principal, resource, action)` is:

```
allow = layer1_allows(principal, resource.scope, action)
     OR any_grant(principal.user ∪ principal.teams, resource, action)
```

So "team X can write dashboards A, B, C; read D" is four grant rows on team X; a
member of team X with **no** org-level write still gets exactly that. A team that
is "editor of all of site Y" needs **zero** grant rows — Layer 1 covers it.

### Identity: users, teams, memberships

| Entity      | Key fields | Purpose |
| ----------- | ---------- | ------- |
| `users`     | id, subject (OIDC `sub` / PAT id), org, email, display_name, status | The human/account. `subject` links the verified token to a user row. |
| `teams`     | id, org, slug, name | A named group within an org. |
| `memberships` | user_id, team_id | Many-to-many: a user in N teams. |
| `grants`    | id, subject_kind+subject_id, resource_kind, resource_id, scope (org/site), permission | Layer-2 ACL. Subject = user or team. |

A principal's **effective grants** = direct (`user:<id>`) ∪ team
(`team:<id>` for every team the user is in). Computed per request, cached per
request (one query joining memberships → grants).

`users.org` is the user's home org; a **super-admin** is a user with the global
(unscoped) admin role and no single org.

### Admin tiers (the three you named)

A new `admin_level` on the user (or derived), gating the management surfaces:

| Tier         | Scope | Can manage |
| ------------ | ----- | ---------- |
| **super-admin** | global (no org) | orgs, all users, all teams, all grants, all sites — everything, every org |
| **org-admin**   | one org | that org's users, teams, sites, grants, dashboards/flows/rules |
| **member**      | by grants | only what Layer-1 scope + Layer-2 grants allow |

Tiers fold into the existing `Role` ladder rather than a parallel concept:
`Role::Admin` (new) at **global scope** = super-admin; `Role::Admin` at **org
scope** = org-admin; `Operator`/`Viewer` = member with write / read defaults.
`Service` stays (machine accounts). This keeps the gate's `covers` logic intact —
admin is "write-capable role whose scope covers the management target," extended
so an `Admin` role additionally unlocks the user/team/grant management routes.

## How it extends the existing code

- **`Scope`** ([scope.rs](../../crates/rubix-server/src/auth/scope.rs)) — `team`
  stops being a free string. `team` in a token claim is resolved to team-id
  memberships; `covers_resource` is unchanged (still org/site), and grants do the
  team-level work. (Keeping `covers_resource` org/site-only is deliberate: teams
  grant via Layer 2, not by widening the scope cover.)
- **`Role`** ([principal.rs](../../crates/rubix-server/src/auth/principal.rs)) —
  add `Admin`. `can_write()` includes it; a new `can_admin()` gates management.
- **`Principal`** gains `user_id: Option<Uuid>` and `team_ids: Vec<Uuid>` (resolved
  at verify time from the subject). `None`/empty for pure-token service accounts.
- **The gate** ([gate.rs](../../crates/rubix-server/src/auth/gate.rs)) — keep
  `authorize_*`; add `authorize_resource(store, kind, id, action)` that runs the
  two-layer decision. The existing scope_auth helper
  ([scope_auth.rs](../../crates/rubix-server/src/api/scope_auth.rs)) becomes Layer 1
  of that, so dashboards/boards/rules pick up grants by switching their call.
- **Verify** ([verify.rs](../../crates/rubix-server/src/auth/verify.rs)) — after a
  PAT/JWT resolves a subject, look up the `users` row + memberships to populate
  `user_id`/`team_ids`. Absent a user row (pure service PAT), the principal is
  token-scope-only (Layer 1 just like today) — fully backward compatible.

## Status

Increments **A–E are shipped**. The model below is implemented end-to-end on
both store backends (SQLite + Postgres `--features cloud`):

- **A — done.** `GET /api/v1/whoami` ([whoami.rs](../../crates/rubix-server/src/api/whoami.rs))
  returns the resolved principal plus `can_write`/`can_admin`; the UI reads it via
  `useWhoami()` and gates the admin nav + surfaces on `can_admin`.
- **B — done.** `users`/`teams`/`memberships` tables (schema v4 migration), store
  CRUD ([users.rs](../../crates/rubix-server/src/store/users.rs),
  [teams.rs](../../crates/rubix-server/src/store/teams.rs)) on both backends, and
  CRUD routes under `/api/v1/orgs/{org}/{users,teams}` + team members. Verify
  resolves `subject → user` and populates `Principal.user_id`/`team_ids`.
- **C — done.** `grants` table + the two-layer `authorize_resource` decision in
  [scope_auth.rs](../../crates/rubix-server/src/api/scope_auth.rs). Dashboard,
  board, and rule read+write gates run the two-layer check; grant CRUD lives
  under `/api/v1/orgs/{org}/grants` and the convenience `…/dashboards/{id}/grants`.
- **D — done.** `Role::Admin` + `can_admin()`; `RequestPrincipal::require_admin`
  gates every management mutation. super-admin = global `Admin`, org-admin =
  org-scoped `Admin`, folded in from `users.admin_level` at verify time.
- **E — done.** UI Members/Teams/Access surfaces under `/o/$org/settings/`,
  gated on `whoami.can_admin`.

### Decisions taken (resolving the open questions below)

- **Admin tier storage**: a `users.admin_level` column (`none|org_admin|
  super_admin`), folded into `Role::Admin` at verify time — admin-ness is a
  property of the identity, not the token.
- **Grant addressing**: textual `resource_kind` + `resource_ref`
  (`dashboard:<uuid>`, `board:<org>/<site_id|->/<slug>`,
  `rule:<org>/<site_id|->/<name>`, or `*` for all-of-kind within the org).
- **Bootstrap**: explicit `RUBIX_SUPERADMIN_SUBJECT` env only. The "first user on
  an empty table is super-admin" fallback was **rejected** — it would silently
  elevate every scoped operator on a fresh deployment (which has no users yet) to
  global admin, dissolving tenant confinement until the first user row lands.
- **Auth-off + management routes** *(deviation from the third guardrail below)*:
  `require_admin` demands a **real** principal, so management mutations return
  403 when auth is off, rather than the open-by-default no-op the resource gates
  use. Resource reads/writes and `whoami` keep their no-op/synthetic behavior, so
  the dev UI still renders the surfaces (their writes just fail closed). This is
  deliberate: a dev/edge server should not let an unauthenticated caller rewrite
  identity/authorization.

## Proposed direction (increments, smallest-blast-radius first)

### A. `whoami` + UI permission-awareness *(no new tables — ship first)*
`GET /api/v1/whoami` returns the resolved `Principal` (subject, scope, role,
later user/teams). The UI reads it once at boot into an auth store, so it can
show/hide admin nav and render "you have read-only here" states. Lands cleanly
today (auth-off returns a synthetic global-admin principal so dev is unchanged).
**This is the foundation everything else renders against.**

### B. Users, teams, memberships *(identity)*
The three tables + CRUD under `/api/v1/orgs/{org}/{users,teams}` and
`/api/v1/orgs/{org}/teams/{id}/members`. `subject → user` resolution in verify.
Org-admin-gated. No behavior change to resource access yet (grants come next).

### C. Per-resource grants *(the ACL)*
The `grants` table + `authorize_resource` two-layer decision + CRUD under
`/api/v1/orgs/{org}/grants` (and a convenience `…/dashboards/{id}/grants`). Switch
dashboard/board/rule read+write gates to `authorize_resource`. This is where
"team X → write dashboard A" becomes real. List endpoints filter to readable
resources (already the pattern; the filter predicate just becomes the two-layer
check).

### D. Admin role + tiers *(management gating)*
Add `Role::Admin`, `can_admin()`, super-admin vs org-admin by scope. Gate B's and
C's management routes on it. Define the bootstrap: the first user in a fresh
deployment (or `RUBIX_SUPERADMIN_SUBJECT`) is super-admin.

### E. UI admin surfaces *(the section that's "missing")*
Under `/o/$org/settings/`: **Members** (users in the org, their teams, role),
**Teams** (create, members, base role), **Access** (per-resource grants — pick a
dashboard/flow/rule, pick teams/users, set read/write). A super-admin gets a
top-level **Orgs** admin. All gated on the `whoami` admin level from increment A.

## Open questions

- **Grant inheritance.** Does a `read` grant on an *org-overview* dashboard imply
  read on the sites it spans? Proposed: no — grants are per-resource; the overview
  is one resource. (Layer 1 org-read already covers the broad case.)
- **Wildcard grants.** Support `resource = *` within a scope (e.g. "team X writes
  all dashboards in site Y") so a team-as-site-editor needs one row, not N?
  Proposed: yes — it collapses the common bulk case and is cheap to match.
- **OIDC group → team mapping.** When OIDC is on, do JWT `groups` claims
  auto-map to teams, or are memberships managed only in-app? Proposed: in-app
  first (deterministic); add a claims→team sync later behind config.
- **Bootstrap super-admin.** First-user-wins vs an env var
  (`RUBIX_SUPERADMIN_SUBJECT`)? Proposed: env var (explicit, reproducible), with
  first-user fallback only when the users table is empty AND auth is on.

## Guardrails this introduces (for AGENTS.md once accepted)

- **Authorization is two-layer and additive.** A resource action is allowed iff
  Layer-1 scope-role covers it OR a Layer-2 grant (direct or via a team) grants
  it. Grants never subtract. Enforcer: `authorize_resource`.
- **Management routes require `can_admin()` at a covering scope.** super-admin =
  global Admin; org-admin = org-scoped Admin. No user/team/grant mutation without it.
- **Auth-off stays a no-op — except management mutations.** With no principal,
  the *resource* gates (`authorize_resource`, read/write) pass and `whoami`
  returns a synthetic global admin, so the dev UI is unchanged. The **management
  gate `require_admin` is the exception**: it demands a real principal and so
  returns 403 with auth off, so an unauthenticated dev/edge caller cannot mutate
  users/teams/grants. Enforcer: `RequestPrincipal::require_admin`.

## Verification

1. Backend builds both backends; clippy clean; migration adds tables idempotently
   (follow the v3 partial-index precedent in `store/migrate.rs`).
2. Store/api tests: a member of team X with no org write can read dashboard A and
   write dashboard B (grants), but is 403 on dashboard C; an org-admin manages
   teams/grants; a super-admin crosses orgs; a Viewer-equivalent is read-only.
   Cross-tenant denial still holds (a user in org A cannot touch org B).
3. `whoami` returns the right principal under auth-on (PAT + JWT) and a synthetic
   admin under auth-off.
4. UI: tsc/lint/build; the Members/Teams/Access pages gate on `whoami`; a
   read-only user sees disabled write controls, not 500s.
