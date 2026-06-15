# Admin & Management API — Principals, Grants, Tenants, Devices

Design for the control-plane HTTP surface that manages identities, capability
grants, tenant namespaces, and edge devices. Reads against [SCOPE.md](../SCOPE.md):
*"Everything is a scoped principal"* (§5), *"Commands go through the gate"* (§7),
one binary edge-to-cloud (§1). This surface fills the gap where the runtime can
*authenticate* principals and *resolve* tenants but has **no HTTP path to create,
list, or revoke** any of them — provisioning today is seed/library-only.

This document is the contract to approve before implementation. It adds no new
storage engine and no new architecture: **every mutation crosses the access gate
as a `Command` and is captured in the audit log with a correlation id** — the same
path record writes take. There are no owner-handle side writes.

## Scope

| Surface | Status today | This adds |
| --- | --- | --- |
| **Principals** (users + extensions) | `provision_principal` + `authenticate` only; library-only | full CRUD over HTTP, gate-audited |
| **Grants** (capabilities) | `create_grant` / `revoke_grant` / `list_grants` exist | HTTP surface, gate-audited |
| **Tenants** (namespaces) | resolved per request; compile-time const set | cloud runtime onboarding + a tenant registry record |
| **Devices** (edge registry) | does not exist | new entity + CRUD + `device-manage` capability |

Principals and grants are one identity model (SCOPE §5) — users and extensions are
the same `Principal`; "user management" and "extension management" are the same
endpoints, distinguished only by `PrincipalKind`.

## Authorization model

Every mutation is guarded two ways, matching the existing record/datasource path:

1. **Admin-role-in-namespace** (transport guard). All mutations require
   `auth.principal.role == Role::Admin && auth.principal.namespace == <target
   namespace>`, else `403 Forbidden`. This mirrors the gate's own `may_administer`
   (`crates/rubix-gate/src/capability/grant/authority.rs:26`), which already governs
   grant creation. The one exception is **tenant onboarding** (see Surface 3) — a
   fresh namespace has no Admin yet, so onboarding is authorized by a **root/system
   principal**, not an in-namespace Admin.
2. **Capability grant** (gate guard, where applicable). Device mutations cross the
   new `device-manage` capability; grant mutations cross the gate's existing grant
   authority. Fail-closed — a missing grant is `403`, never a silent allow.

Authorization stays in the handler via the `Authenticated` extractor
(`crates/rubix-server/src/auth.rs`); no new middleware. Admin endpoints operate on
`auth.principal.namespace` — the principal record's `namespace` field, which the
gate binds to `$auth.namespace` and the row permissions scope reads by. That makes
the in-namespace Admin rule correctly per-tenant by construction (Open item 3): a
tenant's admin only ever sees and administers its own namespace, whichever
SurrealDB namespace the connection signed into.

## Cross-cutting: the gate boundary (audit + correlation)

Resolved from review: **all four surfaces mutate through `rubix_gate::apply` with a
`Command`**, not through owner-handle `provision_principal` / direct grant
`upsert`. Consequences, all deliberate:

- Each create/update/delete produces an **audit record** (principal, namespace,
  action, before/after, correlation id, timestamp) — admin actions are accountable
  by construction, the same as record writes.
- The gate mints the **correlation id**, so an identity/grant/device change is
  traceable end-to-end.
- New gate command variants are required for the writes that do not exist as
  commands today (principal create/delete/role, grant set/revoke as commands,
  device CRUD, tenant-registry write). `provision_principal` and the grant verbs
  remain as the *seed/library* path; the HTTP path wraps the same effect in a
  `Command` so it is audited. This is the one piece of new gate surface beyond the
  principal read verbs.
- **Undo**: identity and grant mutations are audited and reversible-in-principle;
  whether they enter the user-facing undo stack is deferred (Open item 4) — audit
  is unconditional, undo is opt-in per SCOPE's undo boundary (definitions/config).

## Surface 1 — Principals  (`/principals`)

| Method | Path | Auth | Returns |
| --- | --- | --- | --- |
| POST | `/principals` | Admin@ns | `201` + `PrincipalDto` |
| GET | `/principals` | Admin@ns | `200` + `Vec<PrincipalDto>` (namespace-scoped) |
| GET | `/principals/:subject` | Admin@ns | `200` + `PrincipalDto` / `404` |
| PATCH | `/principals/:subject` | Admin@ns | `200` + `PrincipalDto` (role only) |
| DELETE | `/principals/:subject` | Admin@ns | `204` / `404` |

**Subject keying.** The storage key is the **global** record id — `PrincipalRow`
keys by `principal.subject` alone, no namespace (`row.rs:28`). To give each tenant
its own subject space, the **API subject is local-to-namespace**: a request subject
`alice` in namespace `tenant_acme` is stored under the global key
`tenant_acme_alice`. The API accepts and returns the *local* subject; the
`{namespace}_{subject}` prefix is the storage key only. (Hyphens in subjects are
safe — the access method uses `type::record('principal', $subject)`,
`permission/define.rs:34`; the prefix separator is cosmetic, chosen as underscore
to match the seed.)

- **Create** provisions the principal (subject, kind, role, secret) under the
  prefixed key through the gate's audited `create_principal` verb (audit row +
  correlation id). The `secret` is optional — omit it to have the server mint and
  return one (Open item 5). Re-creating an existing subject → `409` (provision is
  non-idempotent).
- **List/Get/Delete/Patch** need read/delete verbs that do not exist today
  (`PrincipalRow` is `pub(crate)`; only create+authenticate are public). **New gate
  verbs**, keeping the `principal` table gate-owned:
  - `list_principals(db, namespace) -> Result<Vec<Principal>>`
  - `get_principal(db, namespace, subject) -> Result<Option<Principal>>`
  - `delete_principal(db, namespace, subject) -> Result<()>` (as a Command)
  - `set_principal_role(db, namespace, subject, role) -> Result<Principal>` (as a Command)
  All filter by namespace and strip the secret.
- **Last-admin guard.** PATCH (demote) and DELETE refuse to remove the **final
  Admin** in a namespace — checked before the write, returns `409 Conflict`. This
  prevents self-lockout; recovery from zero-admin is the root/onboarding path only.
- **PrincipalDto** = `{ subject, namespace, kind, role }` — secret is never returned.

## Surface 2 — Grants  (`/principals/:subject/grants`)

Grants are nested under their principal — a grant has no identity apart from
`(namespace, subject, capability)`.

| Method | Path | Auth | Returns |
| --- | --- | --- | --- |
| GET | `/principals/:subject/grants` | Admin@ns | `200` + `Vec<GrantDto>` |
| PUT | `/principals/:subject/grants/:capability` | Admin@ns | `200` + `GrantDto` (idempotent) |
| DELETE | `/principals/:subject/grants/:capability` | Admin@ns | `204` (idempotent) |

- **Target must exist.** `create_grant` accepts a `Principal` and does **not** verify
  it exists (`create.rs:36`), so the route would otherwise create orphan grants. The
  handler **loads the principal first via `get_principal`** and returns `404` for an
  unknown subject before granting.
- Backed by the existing grant effect (`create_grant`/`revoke_grant`/`list_grants`),
  wrapped in a gate `Command` so the grant change is audited. The gate's internal
  authority check enforces admin-in-namespace; `GrantDenied → 403`.
- `:capability` parsed via `Capability::parse` (`kind.rs:94`); unknown → `400`.
- PUT (grant) is idempotent (upsert); DELETE (revoke) on an absent grant is a no-op
  `204`.
- **GrantDto** = `{ subject, namespace, capability }`.

## Surface 3 — Tenants  (`/tenants`)  — cloud onboarding

A tenant is a **namespace**, not a domain entity (SCOPE non-goal: no tenant schema).
"Create a tenant" means **bootstrap a namespace** plus write one lightweight
**tenant registry record** so the namespace is discoverable.

| Method | Path | Auth | Edge | Cloud |
| --- | --- | --- | --- | --- |
| POST | `/tenants` | root/system | `409 Conflict` | bootstrap + registry write, `201` |
| GET | `/tenants` | root/system | single-namespace | list registry records |
| DELETE | `/tenants/:id?confirm=:id` | root/system | `409 Conflict` | purge namespace + deregister, `204` |

- **One binary, edge to cloud** (SCOPE §1): routes are **always mounted**, never
  `#[cfg]`-gated, so the route table is identical on every build. Handlers branch on
  `state.profile.is_multi_tenant()` (`profile/define.rs:55`); on edge, mutation
  returns `409 Conflict` with a clear message.
- **Onboarding authority.** A fresh namespace has no Admin, so the admin-in-namespace
  rule cannot apply. POST/GET/DELETE `/tenants` are authorized for a **root/system
  principal** (a principal whose role/identity is recognized as system-level), not a
  tenant Admin. Onboarding's job includes provisioning the tenant's **first Admin**.
  This is the documented exception to the §Authorization rule, resolved before
  approval (was Open item 2).
- **Bootstrap** = create the namespace, seed its meta-collection
  (`rubix_core::bootstrap_meta_collection`, as `seed/mod.rs:75`) and the gate/audit
  schema, provision the first Admin, then write the tenant registry record — all as
  gate Commands. Namespace name from
  `state.profile.resolve_namespace(&state.namespace, Some(tenant))`
  (`resolve_tenant.rs:27`), never the private formatter.
- **Tenant registry record (source of truth for GET).** One record per onboarded
  tenant: `{ id, namespace, created_at, first_admin_subject }`. This is a *registry*,
  not a domain schema — it carries no tenant *data*, so it does not violate the "no
  tenant table" non-goal (same distinction datasources use). Chosen over deriving
  from SurrealDB namespace metadata (which couples to engine internals and would list
  system namespaces) or from principals (which hides a freshly-bootstrapped, empty
  namespace).
- **TenantDto** = `{ id, namespace, created_at }`.

## Surface 4 — Devices  (`/devices`)  — edge registry

No device entity exists today; `device-actuate` is only a *command* capability
(`kind.rs:38`) whose egress worker is unbuilt. This adds a **device registry row** —
a control-plane registration, distinct from commanding hardware.

| Method | Path | Auth | Returns |
| --- | --- | --- | --- |
| POST | `/devices` | Admin@ns + `device-manage` | `201` + `DeviceDto` |
| GET | `/devices` | Admin@ns | `200` + `Vec<DeviceDto>` |
| GET | `/devices/:id` | Admin@ns | `200` + `DeviceDto` / `404` |
| PATCH | `/devices/:id` | Admin@ns + `device-manage` | `200` + `DeviceDto` |
| DELETE | `/devices/:id` | Admin@ns + `device-manage` | `204` / `404` |

- **New capability `Capability::DeviceManage`** ("device-manage") — governs the
  *registry*, separate from `DeviceActuate` (commands the hardware). Single-file
  change to `crates/rubix-gate/src/capability/kind.rs`: enum variant + `ALL` array
  (length `8 → 9`) + `as_str` arm + length-assertion test.
- **Persistence — concrete contract.** A device is a gate-written record (audited,
  namespace-scoped):
  - record `content.kind` = `"device"` (the collection discriminator).
  - record id = `device:{namespace}_{id}` — namespace-prefixed for the same
    per-tenant isolation as principals; `id` is caller-supplied and unique within the
    namespace (collision → `409`).
  - body: `{ label: String, kind: String, metadata: Map<String, Json> }` — `kind` is
    the device class (free-form, e.g. `"gateway"`, `"sensor"`), `metadata` is an
    open key/value bag (no fixed schema, consistent with SCOPE's "generic, not
    domain-specific").
  - read path: list/get filter by `content.kind == "device"` and namespace via the
    scoped session, like records.
- The device id doubles as the **sync partition key** SCOPE's sync model references
  (SCOPE §sync, Open Q #1) — making that identity explicit.
- **DeviceDto** = `{ id, namespace, label, kind, metadata }`.

## Decisions (locked)

| Decision | Choice | Rationale |
| --- | --- | --- |
| Principal list/delete | new **gate verbs** | keeps the principal table gate-owned |
| Subject keying | API-local subject, **`{ns}_{subject}` storage key** | per-tenant subject space over a global record key |
| Audit boundary | **all admin writes via `Command::apply`** | "every mutation is audited" holds; no owner-handle side writes |
| Device authorization | new **`device-manage`** capability | separates managing the registry from commanding hardware |
| Tenant on edge | routes mounted, **runtime 409** | one binary edge-to-cloud; identical route table |
| Tenant listing | **tenant registry record** | discoverable + onboarding metadata; not an engine-internal derivation |
| Admin guard | **`Role::Admin` in same namespace** (root for onboarding) | mirrors the gate's `may_administer` |
| Grant target | **load principal first**, `404` if missing | no orphan grants |
| Last admin | **demote/delete of final Admin refused** (`409`) | no self-lockout |

## Open items

1. **Tenant deletion** — *resolved.* `DELETE /tenants/:id` purges every gate-owned
   row tagged with the tenant namespace (records, principals, grants — via
   `rubix_gate::purge_namespace`, audited) then deletes the registry record. Gated
   behind a root/system principal, the multi-tenant profile (edge → `409`), and an
   explicit `?confirm={id}` guard so the irreversible action cannot fire by
   accident (`crates/rubix-server/src/http/admin/tenants.rs`).
2. **Root/system principal definition** — *resolved.* A request is root/system
   when its principal is an `Admin` in the server's **configured root namespace**
   (`state.namespace`) — the deployment's own bootstrap identity, the one
   namespace that exists before any tenant. This is the single recognized root
   path; `/tenants` authorizes against it
   (`crates/rubix-server/src/http/admin/tenants.rs`, `require_system`).
3. **Per-tenant auth resolution** — *resolved: correct by the field model.* Tenant
   isolation is by the principal record's `namespace` **field**, which the gate
   binds to `$auth.namespace` at signin and the `record`/`audit` row permissions
   key reads off. A scoped session therefore reads only its tenant's rows
   regardless of which SurrealDB namespace the connection signed into — so admin
   endpoints operating on `auth.principal.namespace` are already correctly
   per-tenant. The signin namespace is infrastructure (the single bootstrapped
   ns/db that hosts the `principal` access method); it is *not* the tenant
   boundary, so no per-request namespace switch is needed. A freshly onboarded
   tenant's admin authenticates immediately because onboarding provisions it in
   that same `principal` table.
4. **Undo enrollment** — *resolved: audit-only.* Audit is unconditional and is in
   place for every admin mutation. The user-facing undo stack
   (`rubix_gate::UndoStore`) is a per-session in-memory consumer with **no HTTP
   surface today** — records themselves go through `apply` audit-only, with no
   server-side undo wiring. Admin identity/grant/device mutations follow the same
   precedent: audited, not undo-enrolled. Undo enrollment is revisited if and when
   an undo HTTP surface is built (it would enroll records and admin definitions
   together, behind the SCOPE definition/config boundary).
5. **Secret on principal create** — *resolved.* The request secret is now
   **optional**: when supplied it is used (and never echoed back); when omitted the
   server mints a random secret and returns it once in the create response
   (`CreatedPrincipalDto.secret`) — the only response that ever carries a secret.
