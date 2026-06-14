# Feature — Authorization & RBAC (Users, Teams, Grants, Admin Tiers)

> Verified: **increments B–E landed** — real `users`/`teams`/`memberships`/`grants`
> tables (schema v4), two-layer additive authorization, and the admin tiers
> (super-admin / org-admin) on `rubix-gaps`, 2026-06-13. Covered by
> `crates/rubix-server/tests/api_tests/rbac.rs` (7 behavior tests, auth-on PAT
> path) and `crates/rubix-server/tests/migrate.rs` (v4 idempotency), both store
> backends. Source: `rubix-server/src/api/{users,teams,grants}/`, the two-layer
> gate `rubix-server/src/api/scope_auth.rs`, `rubix-server/src/auth/{principal,
> admin_level,verify,gate}.rs`, the concrete + Postgres stores under
> `rubix-server/src/store/{users,teams,grants}.rs`, and UI `ui/src/features/admin/`.
> Design intent: [../../../docs/design/authz-rbac.md](../../../docs/design/authz-rbac.md).

Covers **who, within a tenant, may do what** — layered on top of the org/site
tenant boundary (see [ENTITY_CRUD_AND_TENANCY.md](ENTITY_CRUD_AND_TENANCY.md),
which draws the boundary; this decides access *within* it). Two layers, additive:
**scope-role** (the cheap common case) **OR** a **per-resource grant** (the
precise override) — the Grafana/Niagara "select exactly what each user or team
can see and edit" model. Plus three **admin tiers** that gate the management
surfaces themselves.

Prereq: stack up per [../00_setup/QUICKSTART.md](../00_setup/QUICKSTART.md).
`$BASE`, `post()`, `del()` from
[../reference/API_CHEATSHEET.md](../reference/API_CHEATSHEET.md).

---

## What to prove

Four properties, each a section below.

1. **Identity exists.** `users`, `teams`, `memberships` are first-class and
   round-trip through CRUD. A token's verified `subject` resolves to a `user`
   row, which carries the user's teams and admin tier onto the request principal.
2. **Two-layer additive authorization.** A resource action is allowed iff
   Layer-1 scope-role covers it **OR** a Layer-2 grant (direct `user:<id>` or via
   a `team:<id>`) grants it. Grants **add** access, never subtract. A member with
   *no* org write can still write exactly the resources granted to their team.
3. **Admin tiers gate management.** `require_admin` lets a super-admin (global
   `Admin`) manage every org, an org-admin manage only its own, and refuses
   everyone else. Cross-tenant denial holds (an org-A admin can't touch org B).
4. **Auth-off deviation.** Resource gates stay a no-op with auth off, but the
   **management** mutations (users/teams/grants) return `403` — a deliberate
   exception so an unauthenticated dev/edge caller can't rewrite authorization.

> **Requires auth ON for Parts 1–3.** Mint PATs via `POST /api/v1/tokens` and seed
> matching `users` rows (the PAT id is the verified `subject`). With auth **off**
> (edge default) the resource gates are no-ops, so the grant/scope checks prove
> nothing — only Part 4 is observable. The `rbac.rs` suite drives the auth-on PAT
> path directly; this runbook mirrors it over curl.

---

## Runbook — Part 1: Identity (users, teams, memberships)

All routes here are **admin-gated** (`require_admin` over the path `{org}`), so
run them as a super-admin or org-admin token (`$ADMIN`).

```bash
# Users — CRUD under the org. `subject` is the verified token id / OIDC sub the
# user is keyed by; `admin_level` ∈ none|org_admin|super_admin (default none).
UID=$(curl -s -X POST "$BASE/api/v1/orgs/acme/users" -H "authorization: Bearer $ADMIN" \
  -H content-type:application/json \
  -d '{"subject":"pat-alice","email":"alice@acme.test","display_name":"Alice","admin_level":"none"}' | jq -r .id)
curl -s "$BASE/api/v1/orgs/acme/users" -H "authorization: Bearer $ADMIN" | jq '[.[].email]'

# Teams — a named group within the org.
TID=$(curl -s -X POST "$BASE/api/v1/orgs/acme/teams" -H "authorization: Bearer $ADMIN" \
  -H content-type:application/json -d '{"slug":"ops","name":"Operations"}' | jq -r .id)

# Membership — add the user to the team (idempotent), list, remove.
curl -s -o /dev/null -w "add-member %{http_code}\n" -X POST \
  "$BASE/api/v1/orgs/acme/teams/$TID/members" -H "authorization: Bearer $ADMIN" \
  -H content-type:application/json -d "$(jq -nc --arg u "$UID" '{user_id:$u}')"   # → 204
curl -s "$BASE/api/v1/orgs/acme/teams/$TID/members" -H "authorization: Bearer $ADMIN" | jq '[.[].email]'
```

✅ Users/teams create/list/get/patch/delete round-trip; memberships add (idempotent
`204`)/list/remove. `org`/`subject` on a user and `org`/`slug` on a team are
**immutable** identity (PATCH edits `email`/`display_name`/`admin_level` on a user,
`name` on a team).
✅ **Subject → principal.** After a PAT verifies, `verify.rs` resolves its subject
to the `users` row and populates `Principal.user_id` + `team_ids`, and elevates
the role per `admin_level`. A subject with **no** user row stays a pure-token,
scope-only principal — fully backward compatible with service PATs.
✅ **Privilege ceiling.** An org-admin cannot mint or promote a `super_admin`
(only a super-admin can) → `403`.

## Runbook — Part 2: Two-layer additive grants (the headline)

The discriminating scenario: a team member with **no org write** reads dashboard
A (team read grant), **writes** dashboard B (team write grant), but is `403` on
dashboard C (no grant). `$MEMBER` is a `viewer`-role PAT whose user is in team
`ops`; `$ADMIN` seeds the grants.

```bash
# Seed three org-overview dashboards as admin.
A=$(post /api/v1/dashboards '{"org":"acme","slug":"a","title":"A"}' | jq -r .id)
B=$(post /api/v1/dashboards '{"org":"acme","slug":"b","title":"B"}' | jq -r .id)
C=$(post /api/v1/dashboards '{"org":"acme","slug":"c","title":"C"}' | jq -r .id)

# Grant team `ops` READ on A and WRITE on B (nothing on C). The convenience
# per-dashboard endpoint implies resource_kind/ref:
grant() { curl -s -X POST "$BASE/api/v1/dashboards/$1/grants" -H "authorization: Bearer $ADMIN" \
  -H content-type:application/json -d "$(jq -nc --arg t "$TID" --arg p "$2" '{subject_kind:"team",subject_id:$t,permission:$p}')"; }
grant "$A" read
grant "$B" write

# As $MEMBER (viewer, no org write):
curl -s -o /dev/null -w "write-B %{http_code}\n" -X PATCH "$BASE/api/v1/dashboards/$B" \
  -H "authorization: Bearer $MEMBER" -H content-type:application/json -d '{"title":"B edited"}'   # → 200 (team write grant)
curl -s -o /dev/null -w "write-A %{http_code}\n" -X PATCH "$BASE/api/v1/dashboards/$A" \
  -H "authorization: Bearer $MEMBER" -H content-type:application/json -d '{"title":"A edited"}'   # → 403 (read grant only)
curl -s -o /dev/null -w "write-C %{http_code}\n" -X PATCH "$BASE/api/v1/dashboards/$C" \
  -H "authorization: Bearer $MEMBER" -H content-type:application/json -d '{"title":"C edited"}'   # → 403 (no grant)
```

✅ **Grants ADD, never subtract.** B is writable purely via the team grant despite
the member having no org write. A (read grant only) and C (no grant) are `403` on
write. A `write` grant satisfies `read`; `admin` satisfies both (permission
ordering `admin ⊇ write ⊇ read`).
✅ **Direct grants too.** `subject_kind:"user"` with the user's id works the same as
a team grant; a member otherwise *blind* to a resource (no Layer-1 read) becomes
able to see it via a direct read grant — the list filter is the same two-layer
predicate, so a granted resource appears in the list and an ungranted one is
hidden (`404` on get).
✅ **Wildcard.** `resource_ref:"*"` within an org grants the kind in bulk ("team X
writes all dashboards in acme") — one row, not N. Use the generic
`POST /api/v1/orgs/{org}/grants` for `*` and for board/rule kinds.
✅ **Boards & rules** run the same two-layer gate. Their grant ref is
`board:<org>/<site_id|->/<slug>` / `rule:<org>/<site_id|->/<name>` (`-` = org-level);
create authorizes against the `*` wildcard since there is no id yet.

## Runbook — Part 3: Admin tiers + cross-tenant denial

```bash
# An org-admin of acme manages acme but not org `other`.
curl -s -o /dev/null -w "acme-teams %{http_code}\n" "$BASE/api/v1/orgs/acme/teams"  -H "authorization: Bearer $ORG_ADMIN"   # → 200
curl -s -o /dev/null -w "other-teams %{http_code}\n" "$BASE/api/v1/orgs/other/teams" -H "authorization: Bearer $ORG_ADMIN"  # → 403

# A super-admin (global Admin) crosses every org.
curl -s -o /dev/null -w "super-other %{http_code}\n" "$BASE/api/v1/orgs/other/teams" -H "authorization: Bearer $SUPER"      # → 200

# whoami reflects the resolved principal (role folds in admin_level, scope global for super).
curl -s "$BASE/api/v1/whoami" -H "authorization: Bearer $SUPER" | jq '{role,can_admin,scope}'  # → {"role":"admin","can_admin":true,"scope":{}}
```

✅ super-admin = global `Admin`; org-admin = `Admin` scoped to its org; member =
`Operator`/`Viewer`. `require_admin(org)` passes only for an `Admin` whose scope
covers the org.
✅ **Cross-tenant denial holds.** An acme-admin reading a user that lives in org
`other` (even by guessing the path org) is `403` — `require_admin` confines, and a
by-id resource must live in the path org (else `404`), so no row leaks across the
tenant line.
✅ `whoami` carries `can_admin` so the UI gates the Members/Teams/Access nav and
surfaces on it.

## Runbook — Part 4: Auth-off deviation (no token)

> Run against the **edge default** (auth off, no token).

```bash
# A resource read is still open (no-op gate)…
curl -s -o /dev/null -w "read-sites %{http_code}\n" "$BASE/api/v1/sites"               # → 200
# …but a management mutation is DENIED (require_admin demands a real principal).
curl -s -o /dev/null -w "create-team %{http_code}\n" -X POST "$BASE/api/v1/orgs/acme/teams" \
  -H content-type:application/json -d '{"slug":"x","name":"X"}'                          # → 403
# whoami still synthesizes a dev admin so the UI renders the surfaces.
curl -s "$BASE/api/v1/whoami" | jq '{subject,can_admin,auth_enabled}'  # → {"subject":"dev","can_admin":true,"auth_enabled":false}
```

✅ Management mutations fail closed with auth off; resource gates and `whoami` keep
their no-op/synthetic behavior. This is a **deliberate deviation** from the
"auth-off is a total no-op" convention, scoped to identity/authorization writes.

---

## Acceptance criteria ("done")

- [x] `users`/`teams`/`memberships` CRUD round-trips; subject→user resolution
      populates `Principal.user_id`/`team_ids` and folds `admin_level` into the role.
- [x] Two-layer additive check: a no-org-write team member reads A, writes B
      (grants), `403` on C; grants add and never subtract; `write`⊇`read`.
- [x] Direct (`user:`) and team (`team:`) grants both apply; list filters to the
      two-layer predicate; `*` wildcard grants in bulk.
- [x] Board/rule read+write gates run the same two-layer check.
- [x] `require_admin`: super-admin crosses orgs, org-admin confined, others `403`;
      cross-tenant by-id access denied.
- [x] An org-admin cannot grant `super_admin` (`403`).
- [x] Auth-off: management mutations `403`, resource gates no-op, `whoami` synthetic.
- [x] Schema **v4** migration adds the four tables idempotently, preserves data,
      bumps `user_version`, and re-opens cleanly (verified on a copy of the live DB).

> **Increments B–E are green.** The full design (real identity, two-layer
> additive authorization, admin tiers folded into `Role::Admin`, UI surfaces) is
> implemented across both store backends and covered by `rbac.rs` + `migrate.rs`.
> Bootstrap is explicit `RUBIX_SUPERADMIN_SUBJECT` only (the first-user fallback
> was rejected — it would silently elevate every scoped operator on a fresh,
> empty-users deployment to global admin).

---

## Gotchas

- **`Role::Admin` now exists.** This supersedes the old "no platform-admin role,
  the cross-org actor is a global-scoped Operator" note in
  [ENTITY_CRUD_AND_TENANCY.md](ENTITY_CRUD_AND_TENANCY.md). super-admin = global
  `Admin`, org-admin = org-scoped `Admin`, derived from `users.admin_level`.
- **admin-ness lives on the identity, not the token.** A user's tier is the
  `users.admin_level` column, folded onto the principal at verify time — re-minting
  a PAT does not change it. A bare token with no user row is scope-only.
- **Grants ADD, never subtract.** There is no deny grant. To remove access you
  remove the grant (or narrow Layer-1 scope) — deny-by-omission, like Grafana.
- **Grant refs are exact strings.** `dashboard:<uuid>`, `board:<org>/<site_id|->/<slug>`,
  `rule:<org>/<site_id|->/<name>`, or `*`. Board/rule refs key on `site_id` (the
  uuid), **not** the site slug — deterministic without a slug lookup.
- **Management mutations are NOT open with auth off** (the deviation). Don't expect
  the usual edge no-op for users/teams/grants writes — they need a real admin.
- **Bootstrap is env-only.** Seed the first admin with `RUBIX_SUPERADMIN_SUBJECT`
  (or a pre-provisioned `users` row). There is no first-user auto-elevation.

## Known issues / fixes

- **A grant subject must live in the grant's org.** Creating a grant for a user/
  team from another tenant → `400` (cross-tenant grant prevented).
- **Auth-off can't prove grant *effect*.** Resource gates no-op with auth off, so
  the grant/scope discrimination is only observable with auth **on** — the live
  edge server proves Part 4 (the deviation) but not Parts 2–3; use `rbac.rs` or a
  PAT-enabled instance for those.
- **UI Access page manages dashboard grants only.** Board/rule grants are
  creatable via the generic `POST /api/v1/orgs/{org}/grants` endpoint, but the
  `/o/$org/settings/access` page currently exposes only the per-dashboard picker.
- **No OIDC group → team sync yet.** Memberships are managed in-app; a JWT
  `groups`-claim→team mapping is deferred behind future config (design doc).
