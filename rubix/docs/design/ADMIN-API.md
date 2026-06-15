# Admin & Management API — Principals, Grants, Tenants, Devices

Design for the control-plane HTTP surface that manages identities, capability
grants, tenant namespaces, and edge devices. Reads against [SCOPE.md](../SCOPE.md):
*"Everything is a scoped principal"* (§5), *"Commands go through the gate"* (§7),
one binary edge-to-cloud (§1). This surface fills the gap where the runtime can
*authenticate* principals and *resolve* tenants but has **no HTTP path to create,
list, or revoke** any of them — provisioning today is seed/library-only.

This document is the contract to approve before implementation. It adds no new
storage engine and no new architecture: every mutation crosses the existing access
gate; every read uses the existing scoped session or owner handle.

## Scope

| Surface | Status today | This adds |
| --- | --- | --- |
| **Principals** (users + extensions) | `provision_principal` + `authenticate` only; library-only | full CRUD over HTTP |
| **Grants** (capabilities) | `create_grant` / `revoke_grant` / `list_grants` exist, public | HTTP surface over the existing verbs |
| **Tenants** (namespaces) | resolved per request; compile-time const set | cloud runtime onboarding (bootstrap a namespace) |
| **Devices** (edge registry) | does not exist | new entity + CRUD + `device-manage` capability |

Principals and grants are one identity model (SCOPE §5) — users and extensions are
the same `Principal`; "user management" and "extension management" are the same
endpoints, distinguished only by `PrincipalKind`.

## Authorization model

Every mutation is guarded two ways, matching the existing record/datasource path:

1. **Admin-role-in-namespace** (transport guard). All four surfaces' mutations
   require `auth.principal.role == Role::Admin && auth.principal.namespace ==
   <target namespace>`, else `403 Forbidden`. This mirrors the gate's own
   `may_administer` (`crates/rubix-gate/src/capability/grant/authority.rs:26`),
   which already governs grant creation. We apply the same rule to principals,
   tenants, and devices for one consistent admin rule.
2. **Capability grant** (gate guard, where applicable). Grant and device mutations
   additionally cross a capability check inside the gate (grants: the existing
   authority check; devices: the new `device-manage` capability). Fail-closed —
   a missing grant is `403`, never a silent allow.

No new auth middleware: authorization stays in the handler via the `Authenticated`
extractor (`crates/rubix-server/src/auth.rs:49`), consistent with every existing
route. An endpoint that omits the extractor is unauthenticated by construction;
all admin endpoints take it.

## Surface 1 — Principals  (`/principals`)

| Method | Path | Auth | Returns |
| --- | --- | --- | --- |
| POST | `/principals` | Admin@ns | `201` + `PrincipalDto` |
| GET | `/principals` | Admin@ns | `200` + `Vec<PrincipalDto>` (namespace-scoped) |
| GET | `/principals/:subject` | Admin@ns | `200` + `PrincipalDto` / `404` |
| PATCH | `/principals/:subject` | Admin@ns | `200` + `PrincipalDto` (role only) |
| DELETE | `/principals/:subject` | Admin@ns | `204` / `404` |

- **Create** replicates the seed flow (`seed/cast.rs:97-123`): build the target
  `Principal`, call `provision_principal(state.store.raw(), &principal, secret)`.
  Subjects are namespaced `{namespace}_{subject}` with an **underscore** (a hyphen
  breaks the access-method SIGNIN id — `cast.rs:88`). Body carries `subject`,
  `kind` (`user`|`extension`), `role`, `secret`.
- **List/Get/Delete/Patch** need gate verbs that do not exist today (`PrincipalRow`
  is `pub(crate)`; only create+authenticate are public). **New gate verbs** —
  added beside `provision_principal`, keeping the `principal` table gate-owned:
  - `list_principals(db, namespace) -> Result<Vec<Principal>>`
  - `get_principal(db, namespace, subject) -> Result<Option<Principal>>`
  - `delete_principal(db, namespace, subject) -> Result<()>`
  - `set_principal_role(db, namespace, subject, role) -> Result<Principal>`
  All filter by namespace (cross-tenant isolation) and never return the secret.
- **PrincipalDto** = `{ subject, namespace, kind, role }` — no secret, ever. Reuses
  the role/kind→string mapping already in `dto/auth.rs:62`.
- `provision_principal` uses `.create()` (not idempotent) — re-creating an existing
  subject maps to `409 Conflict`.

## Surface 2 — Grants  (`/principals/:subject/grants`)

Grants are nested under their principal — a grant has no identity apart from
`(namespace, subject, capability)`.

| Method | Path | Auth | Returns |
| --- | --- | --- | --- |
| GET | `/principals/:subject/grants` | Admin@ns | `200` + `Vec<GrantDto>` |
| PUT | `/principals/:subject/grants/:capability` | Admin@ns | `200` + `GrantDto` (idempotent) |
| DELETE | `/principals/:subject/grants/:capability` | Admin@ns | `204` (idempotent) |

- Backed entirely by **existing public verbs**: `list_grants`, `create_grant`,
  `revoke_grant` (`crates/rubix-gate/src/capability/grant/`). No new gate code.
- The grantor is `auth.principal`; the gate's internal `may_administer` enforces
  admin-in-namespace, surfaced as `GateError::GrantDenied → 403`.
- `:capability` is parsed via `Capability::parse` (`kind.rs:94`); unknown → `400`.
- PUT (not POST) because `create_grant` upserts — idempotent, so PUT is the honest
  verb. DELETE on an absent grant is a no-op `204` (revoke is idempotent).
- **GrantDto** = `{ subject, namespace, capability }`.

## Surface 3 — Tenants  (`/tenants`)  — cloud onboarding

A tenant is a **namespace**, not a stored entity (SCOPE non-goal: no tenant table).
"Create a tenant" means **bootstrap a namespace** so principals/records can be
written into it — it does not add a domain entity.

| Method | Path | Auth | Edge behavior | Cloud behavior |
| --- | --- | --- | --- | --- |
| POST | `/tenants` | Admin@ns | `409 Conflict` | bootstrap namespace, `201` |
| GET | `/tenants` | Admin@ns | single-namespace list | list known tenant namespaces |
| DELETE | `/tenants/:id` | Admin@ns | `409 Conflict` | (guarded; see open item) |

- **One binary, edge to cloud** (SCOPE §1): routes are **always mounted**, never
  `#[cfg]`-gated, so the route table is identical on every build. Handlers branch on
  `state.profile.is_multi_tenant()` (`profile/define.rs:55`). On edge (single
  namespace) mutation returns `409 Conflict` with a clear message — honest, not a
  missing route.
- **Bootstrap** = create the namespace and seed its meta-collection
  (`rubix_core::bootstrap_meta_collection`, mirroring `seed/mod.rs:75`) plus the
  gate/audit schema, then provision the tenant's first Admin principal. Namespace
  name comes from `state.profile.resolve_namespace(&state.namespace, Some(tenant))`
  (`resolve_tenant.rs:27`) — never the private formatter.
- **TenantDto** = `{ id, namespace }`.
- Tenant *deletion* (namespace drop) is destructive and irreversible; gated behind
  admin + an explicit confirmation flag, or deferred. Flagged in Open items.

## Surface 4 — Devices  (`/devices`)  — edge registry

No device entity exists today; `device-actuate` is only a *command* capability
(`kind.rs:38`) whose egress worker is unbuilt. This adds a **device registry row**
— a control-plane registration, distinct from commanding hardware.

| Method | Path | Auth | Returns |
| --- | --- | --- | --- |
| POST | `/devices` | Admin@ns + `device-manage` | `201` + `DeviceDto` |
| GET | `/devices` | Admin@ns | `200` + `Vec<DeviceDto>` |
| GET | `/devices/:id` | Admin@ns | `200` + `DeviceDto` / `404` |
| PATCH | `/devices/:id` | Admin@ns + `device-manage` | `200` + `DeviceDto` |
| DELETE | `/devices/:id` | Admin@ns + `device-manage` | `204` / `404` |

- **New capability `Capability::DeviceManage`** ("device-manage") — governs the
  *registry*, kept separate from `DeviceActuate` (commands the hardware). Adding it
  is a single-file change to `crates/rubix-gate/src/capability/kind.rs`: enum
  variant + `ALL` array (length `8 → 9`) + `as_str` arm + the length-assertion test.
- **Device = namespace-scoped registry row** persisted via the gate command path
  (`Command` + `apply`, like records — auditable), or the datasources registry-row
  template (`save`/`forget`). Command path chosen: device registration should be
  audited and undoable like any definition.
- The device id doubles as the **sync partition key** the SCOPE sync model
  references (SCOPE §sync, Open Q #1) — making that identity explicit instead of
  implicit.
- **DeviceDto** = `{ id, namespace, label, kind, metadata }`.

## Files touched

**rubix-gate**
- `src/capability/kind.rs` — add `DeviceManage` variant (+ `ALL` len, `as_str`, test).
- `src/principal/` — new `list.rs` / `get.rs` / `delete.rs` / `set_role.rs` verbs;
  re-export from `lib.rs`. (`provision.rs`, grants unchanged.)

**rubix-server**
- `src/http/principals/` — `mod.rs` (router) + `create/list/get/update/delete.rs`
  + nested `grants/` (`list/put/delete.rs`).
- `src/http/tenants/` — `mod.rs` + `create/list/delete.rs` (profile-branching).
- `src/http/devices/` — `mod.rs` + `create/list/get/update/delete.rs` + `capability.rs`
  (names `DEVICE_MANAGE` once).
- `src/http/mod.rs` — merge the three new routers.
- `src/dto/` — `principal.rs`, `grant.rs`, `tenant.rs`, `device.rs` + `mod.rs` exports.
- `src/openapi/paths.rs` + `document.rs` — annotation stubs + schema registration
  (OpenAPI is manual; nothing auto-discovered).

**Tests** — per CLAUDE.md cycle (implement → test → verify → commit per section):
gate verb unit tests; per-router integration tests asserting the admin-403 guard,
namespace isolation, idempotency (grant PUT/DELETE), and the edge-409 tenant path.

## Decisions (locked)

| Decision | Choice | Rationale |
| --- | --- | --- |
| Principal list/delete | new **gate verbs** | keeps the principal table gate-owned; no storage-boundary leak into transport |
| Device authorization | new **`device-manage`** capability | separates managing the registry from commanding hardware (`device-actuate`) |
| Tenant on edge | routes mounted, **runtime 409** | one binary edge-to-cloud; identical route table per build |
| Admin guard | **`Role::Admin` in same namespace** | mirrors the gate's existing `may_administer`; one consistent admin rule |

## Open items (resolve during implementation)

1. **Tenant deletion** — dropping a namespace is irreversible. Ship as guarded
   (admin + confirm flag) or defer to a follow-up. Recommend: defer DELETE,
   ship POST/GET first.
2. **First-admin bootstrap** — the very first Admin in a fresh tenant cannot be
   created by an in-namespace Admin (none exists yet). Cloud onboarding's POST
   `/tenants` provisions it; document who may call onboarding (a root/system
   principal, not a tenant Admin).
3. **`Authenticated` + per-tenant resolution** — the extractor does not yet apply
   `profile.resolve_namespace` for the cloud per-tenant path (`auth.rs:100`). Admin
   endpoints operate on `auth.principal.namespace`; confirm that is correct for
   cloud before wiring cross-tenant admin.
4. **Secret handling on principal create** — secret is request-supplied today
   (matches seed). Decide whether the server should generate+return it instead.
