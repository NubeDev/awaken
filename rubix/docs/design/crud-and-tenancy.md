# CRUD & Tenancy — Org/Site Lifecycle and Full Entity CRUD

Scope for completing the entity management surface: closing the missing-`Update`
gap across the domain entities (sites, equips, points, boards, widgets), and
making the org→site tenant model an explicit, manageable thing rather than a
string convention that only exists implicitly on rows. This is the substrate the
admin UI ("CRUD tenants and sites") sits on; without it the UI can only create
and delete, never edit, and "tenant" has no surface of its own.

## Problem

Two related gaps, found by reading every API router, store impl, and the auth
scope layer.

### 1. CRUD is really CRD — there is no Update anywhere

Every domain entity supports Create, Read (list + get), and Delete, but **none
support Update**. Verified across the HTTP routers, the concrete store, and the
Postgres store — there is no `update_*` / `patch_*` method at any layer.

| Entity  | Create | List | Get | Update | Delete | Notes |
| ------- | :----: | :--: | :-: | :----: | :----: | ----- |
| Site    | ✅ | ✅ | ✅ | ❌ | ✅ (cascades) | the tenant boundary |
| Equip   | ✅ | ✅ | ✅ | ❌ | ✅ | |
| Point   | ✅ | ✅ | ✅ | ❌ | ✅ | `write`/`cur` mutate value, not the row |
| Board   | ✅ | ✅ | ✅ | ❌ | ✅ | slug-addressed |
| Spark   | ✅ | ✅ | — | ❌ | ❌ | `ack` only; no get/delete |
| Widget  | ✅ | ✅ | — | ❌ | ❌ | no get/delete/update |
| Token   | ✅ | ✅ | — | revoke | revoke | admin surface |

Consequences today:

- Renaming a site's `display_name`, re-tagging an equip, or fixing a point's
  unit means **delete-and-recreate**. For a site that is a cascade delete of
  every equip, point, history sample, and spark beneath it — a rename should not
  destroy data.
- The dashboard-builder and points UI can pin and create but cannot correct a
  mistake in place. The UI work logged this as a real backend gap (no edit verb
  on the wire), not a UI shortcoming.
- Spark and widget surfaces are thinner still (create/list only): no way to
  delete a stale widget or inspect a single spark by id.

### 2. "Tenant" is a concept with no entity and no lifecycle

Across the codebase a **tenant is the `{org}/{site}` pair** — concretely a
`Site` row, keyed `UNIQUE (org, slug)`. The scoped query layer, the tool scope,
and the RBAC gate all agree on this definition. But:

- There is **no `org` entity**. An org ("kfc") exists only as a string stamped
  on its sites and on the tokens scoped to it. Creating the "first KFC tenant"
  means creating a site with `org: "kfc"`; nothing records that the org itself
  exists, who owns it, or whether it is active.
- There is **no tenant/admin surface**. The only way to provision a new tenant
  is `POST /sites` plus `POST /tokens` with a matching scope, by hand. Listing
  "all tenants" means listing sites and grouping by `org` client-side.
- The role enum is `Operator | Service | Viewer` — there is **no platform-admin
  role**. The only principal that can act across orgs is a *global-scoped*
  (unscoped) Operator, which works but is implicit and unlabelled.

## How tenancy works today (the part to preserve)

The isolation is real and must not regress. A tenant boundary is enforced
independently at three layers, so a correctly-scoped caller for one org cannot
see or touch another's data even through tools or raw SQL:

1. **Domain reads/writes** — `list_sites` and friends filter results through
   `principal.authorize_site_read(org, slug)` before they reach the wire, and
   writes go through `authorize_site_write` (write-capable role + scope cover).
   A scoped caller never learns another org's sites exist.
2. **Scoped SQL** — a `QueryScope { org, site }` session exposes the canonical
   tables as **views pre-filtered to that org/site**. Confinement is structural
   (the view definition), not a rewrite of the caller's SQL, so
   `SELECT * FROM points_cur` can only ever return that tenant's rows.
3. **Tools** — `TenantScope` confines an agent's tool calls to one `{org}/{site}`
   by the same key.

Caveats this design keeps in view: isolation is **RBAC over a shared store**, not
physical partitioning; a **global/unscoped** principal sees everything by design;
and when auth is **disabled** (edge profile) every gate is a no-op. The proposals
below add a lifecycle and an Update verb — they do not change where the boundary
is drawn.

## Proposed direction

Three increments, each independently shippable, smallest-blast-radius first.

### A. Add Update across the domain entities

Add `PATCH /api/v1/{sites,equips,points,boards}/{id}` plus the matching
`update_*` store methods (concrete + Postgres), following the existing
one-verb-per-file router/handler/store pattern. PATCH semantics (partial,
absent field = unchanged) over PUT, so a re-tag does not require resending the
whole row.

**Immutable identity fields.** `org`/`slug` on a site and `path` on an equip and
`slug` on a point are **not editable**, because they compose the point keyexpr
(`org/slug/equip-path/point-slug`) that widgets, history, and tool targets
address by string. Editing them silently orphans every reference. The editable
set is the metadata: `display_name`, `tags`, point `unit`/`kind`. Renaming
identity is a delete-and-recreate (already supported via the cascade) or a
future explicit "move" operation that rewrites child keyexprs atomically — out
of scope here, called out so it is a decision and not an omission.

Fill the thin surfaces while here: `GET`/`DELETE` for widgets (the builder needs
a Remove control), `GET`/`DELETE` for sparks.

Each PATCH reuses `authorize_site_write` for gating and `validate_*` for the
mutated fields — no new auth path.

### B. Make the org/tenant a first-class, listable thing

Two options; the first is recommended because it adds the management surface
without a schema migration or a new isolation boundary.

- **B1 (recommended): derived org view + provisioning endpoint.** Keep `org` as
  the site field it is. Add `GET /api/v1/orgs` that returns the distinct orgs
  the principal can see (derived by grouping visible sites), each with its site
  count and aggregate tags — the "tenant list" the admin UI renders. Add a
  `POST /api/v1/orgs` convenience that provisions a new tenant in one call:
  create the first site under a new org and (optionally) mint a scoped token for
  it, so "onboard KFC" is one action instead of two hand-coordinated ones.
  No new table, no change to the isolation model.

- **B2 (deferred): a real `orgs` table.** A row per org (id, name, slug, status,
  created_at) that sites FK into. Gives orgs their own lifecycle (rename,
  suspend, soft-delete) and a place to hang per-tenant settings. Larger: a
  migration, a store entity, and a backfill from existing `site.org` strings.
  Worth it only once orgs need state of their own (billing, suspension,
  org-level config) — record the trigger, don't build ahead of it.

### C. Name the platform-admin principal

The cross-org admin surface (orgs, tokens, any-site writes) is today an implicit
global-scoped Operator. Make it explicit so the admin UI has something honest to
gate on. Two options:

- **C1 (recommended): label, don't add a role.** Treat *global scope +
  write-capable role* as "platform admin" and expose it as a derived flag on the
  principal the UI can read. No enum change, no token-minting change; matches the
  current trust model exactly (a global Operator already passes every check).
- **C2 (deferred): an `Admin` role.** A real role in the enum, gating
  org/tenant/token CRUD distinctly from site writes. Cleaner separation but
  touches how every token is minted and migrated — defer until C1's "global
  Operator == admin" conflation actually bites.

## Open questions

- **Identity rename.** Is delete-and-recreate acceptable for renaming a site's
  `org`/`slug`, or is an atomic keyexpr-rewriting "move" a near-term need? This
  decides whether B stays metadata-only.
- **Org as entity (B1 vs B2).** Do orgs need state of their own (suspend,
  billing, org-level config) soon? If not, B1's derived view is sufficient and
  B2 is premature.
- **Admin role (C1 vs C2).** Is "global-scoped Operator == platform admin" an
  acceptable conflation, or must platform administration be a role distinct from
  a broadly-scoped operator?
- **Cascade-delete confirmation.** Site delete already cascades to all children.
  The admin UI must surface the blast radius (child counts) before confirming —
  a read the `GET /orgs`/site-detail response should carry.
