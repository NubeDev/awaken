# HOOKS-AND-FILES — build-order steps 5 & 6, as shipped

Implementation note for the last two build-order steps of
[BACKEND-COLLECTIONS.md](BACKEND-COLLECTIONS.md): **write-triggered hooks** (step
5) and **file fields + blob store** (step 6). Steps 1–4 (collection-as-record,
gate validation + strict mode, list/realtime kind+tag filter, auth token
endpoints) were already in the tree; this note records what steps 5 and 6 added,
the decisions taken at the two open forks, and what remains deferred. Where this
note and [SCOPE.md](../SCOPE.md) disagree, SCOPE.md wins.

## Step 5 — write-triggered hooks (after-hooks)

### What shipped

- A **hook dispatcher** ([`rubix-server/src/hooks`](../../crates/rubix-server/src/hooks/mod.rs)):
  a background task that subscribes to the `record` table on the live-query data
  plane and, when a watched record is written, fires the bound rule through the
  gate. Started at boot by `spawn_hook_dispatcher` in
  [`main.rs`](../../crates/rubix-server/src/main.rs).
- The hook *binding* already existed in core (`kind:"hook"` record →
  `rubix_core::Hook` + `find_hooks`); step 5 added only the trigger.
- One seeded demo hook (edit a `site` → re-fire `high-zone-temp`) and integration
  tests proving a watched write fires its rule (recording a gated, audited insight)
  and an unwatched write fires nothing.

### Decision at the fork — after-hooks only

The design flagged the execution model as load-bearing. We chose **after-hooks
only** (async, off the live-query bus): the rule fires *after* the write commits,
so it is a side effect and **cannot veto a write**. This needed no change to the
gate's `apply()` path.

**Before-hooks (rejecting hooks) are deliberately out of scope.** A hook that can
reject a write must run *inside* `apply()` (the gate must not commit, then
un-commit) — a different mechanism, a gate change, not a bus subscriber. If
before-hooks are wanted later, they are their own design: a validate-style step in
[`command/apply.rs`](../../crates/rubix-gate/src/command/apply.rs) between
`validate` and `capture`, not an addition here.

### Mechanics worth knowing

- **The dispatcher runs on its own thread with a current-thread runtime.** The Rhai
  rule engine's evaluation future is `!Send`, so it cannot run on the multi-threaded
  request runtime via `tokio::spawn`. The dedicated thread keeps the engine off the
  request workers entirely.
- **Identity.** Firing a rule records an insight through the gate, which needs the
  `RuleInvoke` grant in the record's namespace. The dispatcher uses a per-namespace
  **system principal** (`{namespace}_system`, an extension service account)
  provisioned lazily on first use. Its secret is rotated each boot
  (`reprovision_principal`, an upsert) and held only in memory — there is no stored
  plaintext credential to leak.
- **Cross-tenant routing.** One dispatcher serves every namespace: it subscribes on
  the gate **owner** handle (sees all tenants' committed writes) and routes each
  change to a system principal **scoped to that change's namespace** before firing,
  so a hook in tenant A can never read or write tenant B's data.
- **Hook cache.** Bindings are cached per namespace and invalidated when a
  `kind:"hook"` record changes (seen on the same stream), so a new hook takes effect
  without a per-write reload.
- **Trace schema at boot.** The dispatcher is the **first runtime caller** of the
  full `rubix_rules::evaluate` path (only side-effect-free `dry_run` was wired
  before), which persists a span tree. `main.rs` now calls `define_trace_schema` at
  boot so a firing can record its span.

### Recursion guard

A fired rule writes an insight record whose `kind` is the rule's `output`. That
write reappears on the live-query stream, so without a guard a hook whose `match`
equals an output kind would fire on the insight it produced and loop forever.

**The guard:** the dispatcher treats **insight writes as non-hookable**. It tracks
the set of every rule's `output` kind per namespace (loaded from the `kind:"rule"`
records, refreshed when a rule changes) and skips any change whose `kind` is in that
set. Since the only records the dispatcher itself creates are insights, and it never
re-triggers on them, a hook→rule→insight loop is **structurally impossible** — not
merely discouraged. Verified by `a_hook_on_a_rules_own_output_does_not_recurse` in
the dispatcher test.

This needs no write-provenance in the stream (which it does not carry). The one
trade-off: a *user* collection whose kind happens to equal a rule's output will not
fire hooks (its writes look like insights). That is a safe-fail — a missed hook,
never a runaway — and rule outputs are conventionally distinct insight kinds
(`high-temperature`), not domain collections. Rule→rule chaining is expressed by
**sub-rule composition**, not by hooking on insight kinds.

### Sizing against SCOPE OQ2

Write-triggered hooks add fan-out to the single-engine pub/sub plane SCOPE open
question 2 flags as the unhatched bet. The dispatcher rides that plane; it does not
introduce a second bus. Hook fan-out must be sized against SCOPE OQ2, not assumed
free.

## Step 6 — file fields + blob store

### What shipped

- A new crate **[`rubix-blob`](../../crates/rubix-blob)**: the `BlobStore` trait, a
  working `LocalFsBlobStore` (the edge default), the `FileRef` reference type, and
  errors. Blobs are keyed by `(namespace, id)`, so tenant isolation is by path.
- A fail-closed **`Capability::FileUpload`** grant (the fourth-plane authority for
  writing bytes), granted to the seed operator.
- Routes **`POST /files`** (multipart upload → returns a `FileRef`) and
  **`GET /files/:id`** (streams bytes, scoped to the caller's namespace), wired into
  the router and the OpenAPI document. The `file` field type and its reference shape
  already existed in `rubix_core`.
- Round-trip, isolation, capability, and not-found tests.

### Decision at the fork — local FS now, gated upload

Chosen: **local-filesystem store now + a fail-closed `FileUpload` capability**. The
two-step contract is preserved — a client uploads bytes (gated on `FileUpload`) and
gets a reference, then stores that reference in a record through the **normal gated
write**, so the command gate still sees only JSON. Bytes never cross the gate.

### What remains deferred (BACKEND-COLLECTIONS open question 8)

These are intentionally **not** shipped, matching the design's own deferral:

1. **Object-store (cloud) backend.** `rubix-blob` carries a `cloud` feature flag and
   the `BlobStore` trait is the seam, but the S3/GCS backend itself is not built.
   Requesting it fails closed (`BlobError::BackendUnavailable`) rather than silently
   degrading — the same pattern as the Postgres connector. The running server uses
   the local-FS store on every profile today.
2. **Orphan GC.** When a record referencing a blob is deleted, the bytes are not yet
   swept. `BlobStore::delete` is idempotent and ready for a sweeper; the policy
   (immediate vs. mark-and-sweep) is open.
3. **Blob ↔ cloud sync.** `rubix-sync` is append-only by edge partition; large blobs
   may need a separate shipping path. How (or whether) bytes participate in
   edge↔cloud sync is unresolved.

## Test surface added

- `rubix-blob`: `tests/local_test.rs` (round-trip, tenant isolation, idempotent
  delete, path-traversal rejection) + `FileRef` unit tests.
- `rubix-server`: `tests/hooks/dispatch_test.rs` (watched write fires its rule;
  unwatched fires nothing) and `tests/http/files/files_test.rs` (upload→download
  round-trip, forbidden without the grant, 404 for an unknown id).
- `rubix-gate`: the `Capability` round-trip/registry tests cover the new
  `FileUpload` variant.
