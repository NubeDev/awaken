# BACKEND-COLLECTIONS — PocketBase-shaped backend on the rubix substrate

Design for the **backend changes** that turn rubix's generic record store into a
PocketBase-shaped serverless backend: runtime-defined collections, per-kind
typed CRUD with validation, file fields, write-triggered hooks/rules, and
filtered realtime — all driven from data and the API, with **no per-domain
backend code**. The UI that consumes this is [ADMIN-UI.md](ui/ADMIN-UI.md); the
agent runtime that shares the substrate is [AGENT.md](AGENT.md). The **scope
authority** is [SCOPE.md](../SCOPE.md) ("the domain is not baked in; structure
comes from tagging on a graph") and the crate contracts are
[STACK-DEISGN.md](../../STACK-DEISGN.md). Where this doc and SCOPE.md disagree,
SCOPE.md wins.

## Thesis

PocketBase's value is not its store — it is that an admin defines a *collection*
(a named shape with typed fields, validation, access rules, and file fields) at
runtime and immediately gets typed CRUD, realtime, and an admin UI for it, with
zero backend code. Rubix already owns the hard substrate PocketBase had to build
(gate + audit + correlation id + scoped-session row perms + live queries +
unified query + OpenAPI). What's missing is the **collection layer**: a way to
say "records of kind X have this shape" and have the generic CRUD path enforce
and exploit it.

The non-negotiable rule from SCOPE — *the domain is not baked in* — is preserved
by making a collection a **record**, not a table. Defining "Sites" or "Tasks"
adds a `kind: "collection"` record through the existing gate; it does not add a
SurrealDB table, a Rust type, or a route. The schemaless store stays schemaless;
collections are an **opt-in contract layer on top**, fail-open to raw JSON when
no collection is registered (so today's behavior is unchanged for unkinded
records).

This is the one gap that converts "generic record store" into "build the
backend/schema from the UI." Everything else (auth, audit, realtime, query) is
already there.

## Where the backend is today (the delta)

Status against the current code (this design has since shipped — the "Delta" column
records what was built, ✅ = landed, ⏳ = still open):

| PocketBase capability | Rubix today | Delta — status |
| --- | --- | --- |
| Runtime-defined collections | ✅ `kind:"collection"` records define typed shapes at runtime ([collection/def.rs](../../crates/rubix-core/src/collection/def.rs), [bootstrap.rs](../../crates/rubix-core/src/collection/bootstrap.rs)) | ✅ collection-as-record + contract layer |
| Generic typed CRUD | ✅ writes validated against the collection schema in the gate path ([gate command/validate.rs](../../crates/rubix-gate/src/command/validate.rs)) | ✅ per-kind validation in the gate write path |
| List filtered by collection | ✅ `GET /records?kind=…&tag=…` filters on top of row perms ([records/list.rs](../../crates/rubix-server/src/http/records/list.rs)) | ✅ kind/tag filter on list |
| Auth (users + service accts) | ✅ `Principal` {subject, namespace, kind, role}, subject+secret auth, scoped sessions ([authenticate.rs:31](../../crates/rubix-gate/src/authenticate.rs#L31)); `POST /auth/login` + `GET /auth/me` | ✅ token/session issuance endpoint |
| File fields | ✅ `file` field stores a `FileRef`; blob store + `POST /files`/`GET /files/:id` ([rubix-blob](../../crates/rubix-blob/src/lib.rs)), gated on `FileUpload` | ✅ blob storage + file-field type |
| Server-side hooks on write | ✅ `kind:"hook"` records fire rules after commit off the live-query plane ([hooks/mod.rs](../../crates/rubix-server/src/hooks/mod.rs)) | ✅ write-triggered hooks ([HOOKS-AND-FILES.md](HOOKS-AND-FILES.md)) |
| Per-collection access rules | ⚠️ row perms (namespace) + capability grants ([capability/kind.rs](../../crates/rubix-gate/src/capability/kind.rs)); per-collection `listRule`/`writeRule` kept as raw JSON, evaluation deferred | ⏳ collection-scoped access expressions |
| Audit / correlation | ✅ immutable audit, one correlation id ([command/apply.rs:51](../../crates/rubix-gate/src/command/apply.rs#L51)) | reuse as-is |
| OpenAPI | ✅ utoipa, all routes ([openapi/document.rs:18](../../crates/rubix-server/src/openapi/document.rs#L18)) | ⏳ per-collection schema components |
| Realtime | ✅ `/ws/records?kind=…` scoped by principal, row-filtered at source ([ws/subscribe.rs](../../crates/rubix-server/src/ws/subscribe.rs)) | ✅ filter the live stream by kind |

The substrate columns marked ✅ are why this is additive, not a rewrite: every
new write still goes through `apply()` (the gate), every read still runs on the
scoped session, and audit/correlation come for free.

## The collection model — a record, not a table

A **collection** is a `kind: "collection"` record whose `content` is a
PocketBase-style definition:

```jsonc
{
  "kind": "collection",
  "name": "site",                  // the kind value records of this collection carry
  "schema": [                       // typed fields → JSON Schema for content validation
    { "name": "key",  "type": "text",   "required": true,  "unique": true },
    { "name": "name", "type": "text",   "required": true },
    { "name": "area", "type": "number" },
    { "name": "plan", "type": "file",   "maxSize": 5_000_000 }
  ],
  "listRule":   "principal.role >= viewer",   // access expressions (open question 3)
  "writeRule":  "principal.role >= operator",
  "indexes":    ["key"]
}
```

Consequences that keep SCOPE intact:

- **No new table.** Collections live in the same `record` table; defining one is a
  normal gate command. Bootstrapping is a `collection`-defining-`collection`
  (PocketBase does the same) — one built-in seed record.
- **`kind` becomes load-bearing convention, not a schema column.** A record
  "belongs to" a collection when `content.kind == collection.name` (and/or carries
  the matching tag). The store stays schemaless; the *contract* lives in the
  collection record, read at the gate.
- **Fail-open is a migration ramp, not a permanent mode.** A record whose `kind`
  matches no collection is admitted as raw JSON exactly as today — this keeps
  backward compatibility with existing unkinded records and lets a tenant adopt
  collections incrementally. But left permanently on, fail-open is a security hole
  *and* it makes validation advisory: anyone can write unvalidated content by using
  a `kind` that matches no collection, or by typo'ing the `kind`. PocketBase is
  fail-closed (no collection ⇒ no endpoint). So the contract layer carries a
  **per-namespace strict-mode switch**: fail-open while a tenant migrates, flipped
  to **fail-closed** once its collections exist (an unknown/typo'd `kind` is then
  *rejected*, not silently admitted). Strict mode is the intended end state; the
  default at first is fail-open only to avoid breaking today's data. This is the
  one decision that determines whether validation is real or cosmetic — see open
  question 1.

## Per-kind validation in the gate write path (the contract layer)

This is the core backend change and it goes in **one place**: the gate's command
apply path, where every create/update already flows
([command/apply.rs:51](../../crates/rubix-gate/src/command/apply.rs#L51)).

- A new **validate** step runs *after authorize, before capture*: resolve the
  target record's `kind` → load its collection record → validate `content`
  against the collection's JSON Schema → reject the command with a typed error if
  invalid. No collection ⇒ skip (fail-open).
- Validation is a `Change`-aware step (full schema on create; the merged result on
  update) so it sits naturally in the existing pipeline next to `correlate` and
  `capture`.
- **Why the gate, not the HTTP handler:** contract #1 says the gate is the single
  mutation chokepoint. Putting validation in the HTTP layer would let the agent
  runtime, sync, and ingest write unvalidated content. One enforcement point.
- Unique/index constraints (`"unique": true`, `indexes`) are realized as
  **SurrealDB `DEFINE INDEX`** emitted when a collection is created/updated — the
  one place we *do* touch the store schema. This is **not cleanly additive** and is
  the one part of the design that can break a running store: adding a unique index
  to a collection whose existing records already violate uniqueness fails (or
  silently does not enforce), and altering/dropping a field's index is a schema
  migration with edge↔cloud sync implications (SCOPE's config-plane conflict
  surface). Index changes therefore need an explicit migration step (validate
  existing records before defining, decide reject-vs-skip on conflict) — promoted
  to open question 10, not hand-waved here.

This converts the "generic CRUD, opaque content" row in the delta table into
"typed CRUD with server-enforced validation" without a per-collection route or
type — the same `POST /records` handler now produces typed, validated records.

## List/realtime filtering by collection

Both the read list and the live stream need a collection filter that does not
exist today.

- **List** — `GET /records?kind=site&tag=...&where=...`: filter by `kind`
  (collection), by tag (the `tagged` graph), and eventually by content fields.
  Simple kind/tag filters stay on the scoped session (row perms still apply on
  top); non-trivial field predicates route through the existing `/query`
  DataFusion surface rather than growing a bespoke filter DSL. Decide the split in
  open question 5.
- **Realtime** — `/ws/records` currently streams every record change the
  principal can read ([ws/subscribe.rs:34](../../crates/rubix-server/src/ws/subscribe.rs#L34)).
  Add a subscribe-time `kind`/`tag` filter so a "Sites" grid wakes only on site
  changes. The filter is applied **on top of** the row-perm scoping, never instead
  of it — it narrows, it cannot widen, so it stays inside contract #1.

## File fields — the one genuinely new subsystem

Nothing for binary storage exists anywhere in the tree, so this is net-new and the
largest single piece.

- **A `file` field type** in the collection schema; its stored value in `content`
  is a **reference** (an id + filename + size + content-type), never the bytes.
- **A blob store boundary** beside `rubix-store`: pluggable backend, **local
  filesystem for edge** (the default, matching the embedded-engine edge profile)
  and an object-store backend for cloud behind the `cloud` feature, failing closed
  when absent — mirroring the Postgres-connector pattern.
- **Upload/download routes** — `POST /files` (multipart) returns a file reference
  the client then stores in a record's content via the normal gated write;
  `GET /files/:id` streams bytes on the scoped session (a file is owned by the
  record's namespace, so row perms decide access). Keeping upload and record-write
  as two steps means the gate still sees only JSON, preserving the
  one-mutation-chokepoint contract; the file's lifecycle (orphan GC when the
  referencing record is deleted) is a follow-up.
- **Audit** of file ops piggybacks on the record write that references them; raw
  blob writes are data-plane, not gate commands (open question 6 — whether file
  upload itself needs a capability).

## Server-side hooks — fire rules on write

PocketBase hooks run on record events. Rubix has the engine
([rubix-rules](../../crates/rubix-rules)) but rules fire **offline, manually
invoked** ([evaluate/mod.rs:97](../../crates/rubix-rules/src/evaluate/mod.rs#L97)) —
there is no write trigger. The wiring already exists to add one: every gate
mutation publishes a data-change event, and rules already record insights through
the gate.

- **Hook binding** — a collection (or a standalone `kind: "hook"` record) declares
  `on: ["create","update"]` + a rule id. A bus subscriber on the **live-query
  data plane** (row-perm scoped — never the ungated in-process plane, same caution
  as [AGENT.md](AGENT.md) "two wake paths") matches write events to hook bindings
  and invokes the rule.
- **The rule still goes through the gate** — its insight write is a `RuleInvoke`
  command, audited and correlated like today. So a hook adds *triggering*, not a
  new write path. No new capability for the rule itself.
- This makes "computed fields", "validation beyond schema", and "side-effects on
  write" expressible as data, matching PocketBase hooks, while staying on the two
  chokepoints.
- **Two caveats that make this not a free addition.** First, write-triggered hooks
  add fan-out load to exactly the single-engine pub/sub plane that **SCOPE open
  question 2** flags as the unhatched bet (no escape hatch from fan-out
  concentration). The hook design rides that risk; it does not introduce a second
  bus, so it must be sized against SCOPE OQ2, not assumed free. Second, the
  sync-vs-async split (open question 8) is **load-bearing, not cosmetic**: a
  "before" hook that can *reject* a write must run **inside `apply()`** (the gate
  must not commit, then un-commit) — so before-hooks are not "off the bus" at all
  and are a different mechanism from after-hooks. Decide explicitly whether
  before-hooks are in scope; if they are, they are a gate change, not a bus
  subscriber.

## Auth — close the session-issuance gap

Auth is real but minimal: subject+secret in `x-rubix-subject`/`x-rubix-secret`
headers, verified against a stored `principal` record
([authenticate.rs:31](../../crates/rubix-gate/src/authenticate.rs#L31)); identity
kinds are `User | Extension` ([principal/kind.rs:16](../../crates/rubix-core/src/principal/kind.rs#L16)).
For a PocketBase-like UX the deltas are:

- **A login endpoint** that exchanges credentials for a short-lived token, so the
  browser/desktop UI isn't shipping a raw secret on every request. Token →
  principal resolution slots in front of the existing `authenticate()` with no
  change to the gate.
- **A "current principal + grants" endpoint** so the UI can reflect capabilities
  (the auth surface ADMIN-UI open question 4 needs).
- **User self-signup/management** as gated CRUD over `principal` records — no new
  subsystem, just collection-style management of the principal kind that already
  exists. Edge stays single-tenant; cloud is namespace-per-tenant, both already
  modeled.

The principal model and real authentication already exist, so this is not an auth
rebuild. But the token endpoint is a **security surface, not ergonomics**: token
format (open question 9), **revocation**, expiry, and **namespace-scoping of
tokens** all have to be right before this ships, or the login endpoint becomes the
weakest link in an otherwise fail-closed gate. For that reason it is **deferred**
(build order step 4 is the *interface*; the token security model is its own short
design note alongside this one) rather than treated as a quick add in front of
`authenticate()`.

## OpenAPI — per-collection schemas

The OpenAPI doc is generated and covers every route
([openapi/document.rs:18](../../crates/rubix-server/src/openapi/document.rs#L18)),
but `content` is documented as opaque `Value`. To get PocketBase-style typed
client generation per collection, emit a **schema component per registered
collection** (derived from its JSON Schema) and reference it from the `/records`
paths when a `kind` is specified. This is what lets `@rubix/api`
([ADMIN-UI.md](ui/ADMIN-UI.md)) generate a typed client per collection. Generated at
runtime from collection records, not hand-written.

## Contracts honored

- **Generic-by-construction (SCOPE principle 4)** — a collection is a record, not
  a table or a type. No domain is baked into the binary; defining a collection is
  a gate command. The store stays schemaless; typing is an opt-in contract layer
  (fail-open during migration, fail-closed under strict mode — open question 1).
- **Two enforcement points (STACK-DEISGN #1)** — validation, file references, and
  hook-driven rules all land on the **existing** chokepoints: mutations through
  `apply()` (the gate), reads/live on the scoped session. No new write path
  escapes the gate.
- **Two authz layers (STACK-DEISGN #2)** — record access stays SurrealDB row perms
  on the scoped session; cross-plane actions stay the five capability grants. New
  surfaces (file upload, collection define) are deliberate, fail-closed decisions,
  not assumed grants (open question 6).
- **Correlation id (#3)** — every new mutation (validated write, hook insight,
  file-referencing write) carries the gate-minted correlation id into audit and
  spans, unchanged.
- **Fail-closed (A-G5 spirit)** — unknown capability denied. The unknown-`kind`
  fail-open path is a *bounded migration ramp*, not a standing exception: a tenant
  in strict mode rejects unknown kinds, restoring fail-closed behavior (open
  question 1). It is the only path that is ever fail-open, and only until strict
  mode is on.
- **SurrealDB does as much as possible (#6)** — validation indexes are
  `DEFINE INDEX`; filtering uses native queries; only blob bytes live outside
  SurrealDB (the one thing it isn't for).

## Build order (smallest load-bearing first)

1. **Collection-as-record + read** — define the `collection` shape, seed the
   bootstrap collection, expose collection records via the existing CRUD. Unblocks
   the UI's schema registry (ADMIN-UI open question 1) with zero gate change.
2. **Gate validate step + strict mode** — per-kind JSON-Schema validation in
   `apply()`, with the per-namespace strict-mode switch (open question 1). The
   strict-mode decision must be settled *before* this lands — it determines whether
   validation is real or cosmetic. Self-contained, well-tested by the existing gate
   test pattern.
3. **List/realtime kind+tag filter** — the read-side narrowing the grids need.
4. **Auth token endpoint + principal/grants read** — the *interface* the UI login
   needs; the token security model (revocation, expiry, namespace-scoping) is its
   own design note and gates the actual ship (open question 9). Deferred behind
   steps 1–3.
5. **Write-triggered hooks** — bus subscriber → rule invoke; reuses both engines.
   Sized against SCOPE OQ2 fan-out; before-hooks (if in scope) are a gate change,
   not a bus subscriber (open question 8). Deferred.
6. **File fields + blob store** — the one net-new subsystem; largest, last.

Steps 1–3 are low-risk, high-leverage and ship as designed. Steps 4–5 each touch a
SCOPE open question and are deferred to their own short design notes. Every step is
independently shippable and leaves the system production-ready (no placeholder
behavior); the contract layer fails open only during migration (strict mode off),
so nothing breaks today's unkinded records while a tenant adopts collections.

## Open questions

1. **Fail-open vs. strict mode (the decision that makes validation real).**
   Fail-open on unknown `kind` keeps backward compat but is a permanent hole if
   left on — anyone bypasses validation with a typo'd or unregistered `kind`. The
   design carries a per-namespace strict-mode switch (fail-closed once collections
   exist). Settle the granularity (per-namespace vs. per-collection), the default
   for a fresh namespace, and the migration path to flip an existing namespace to
   strict. **This blocks build-order step 2** — without it, validation is cosmetic.
2. **Collection bootstrap.** A single built-in `collection`-defining-`collection`
   seed record vs. a tiny hardcoded meta-schema in the gate. PocketBase uses the
   former; decide whether the meta-collection is itself editable.
3. **Validation library.** Which Rust JSON-Schema validator (production-ready,
   maintained) for the gate validate step, and whether collection field types are
   a small closed enum (text/number/bool/date/file/relation) compiled to JSON
   Schema, or raw JSON Schema stored directly.
4. **Per-collection access rules.** PocketBase has list/view/create/update/delete
   rule expressions. Map these onto rubix's two layers: are they row-perm
   expressions pushed into SurrealDB (native, enforced on reads too) or
   gate-evaluated predicates (commands only)? Row-perm is the stronger fit for
   reads; decide the expression language and where it evaluates.
5. **Relations.** Record→record links are JSON fields today, not edges. Add a
   `relation` field type backed by a typed graph edge (extending the `tagged`
   pattern to `record→relates→record`), or keep id-in-content? Relations are the
   other half of PocketBase parity and affect validation, filtering, and the UI.
6. **List filter surface.** Where is the line between a built-in `?kind=&tag=`
   filter on `/records` and routing to `/query` (DataFusion) for field predicates?
   Avoid growing a second query DSL.
7. **File-upload capability.** Does blob upload need a new fail-closed `Capability`
   variant (a deliberate enum change, like AGENT.md's memory-write question), or is
   it admitted as a data-plane write under the namespace scope? It is a mutation
   either way.
8. **Blob backend + lifecycle.** Local FS (edge) vs. object store (cloud feature);
   orphan/GC policy when a referencing record is deleted; how file bytes
   participate (or not) in edge↔cloud sync (`rubix-sync` is append-only by edge
   partition — large blobs may need a separate shipping path).
9. **Hook execution model.** Synchronous (block the write until the hook rule
   runs, PocketBase-style "before" hooks) vs. asynchronous off the bus ("after"
   hooks only). Sync hooks that can *reject* a write must run inside `apply()`,
   not off the bus — decide whether before-hooks are in scope. Also size the
   write-trigger fan-out against **SCOPE open question 2** (single-engine pub/sub
   concentration has no escape hatch yet).
10. **Token security model.** Not just format (opaque server-side session token vs.
    signed JWT) — also **revocation**, expiry, and **namespace-scoping** of issued
    tokens, and the tie-in with the existing principal-record secret. This is a
    security surface that gates build-order step 4 and likely warrants its own short
    design note.
11. **Index/unique migration (can break a running store).** `DEFINE INDEX` for a
    collection's `unique`/`indexes` is *not* cleanly additive: adding a unique index
    over records that already violate uniqueness fails or silently won't enforce,
    and altering/dropping an indexed field is a schema migration with edge↔cloud
    sync implications (config-plane conflict surface). Decide the migration step:
    validate existing records before defining, reject-vs-skip on conflict, and how
    index definitions propagate (or don't) across edge↔cloud.
