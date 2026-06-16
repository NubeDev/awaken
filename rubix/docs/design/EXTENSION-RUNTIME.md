# Extension Runtime — Supervision, Lifecycle, Status & Metrics

Design for the **runtime half** of the extension system: the layer that actually
starts, stops, supervises, and reports on extensions — and an HTTP admin surface to
drive it. Reads against [SCOPE.md](../SCOPE.md): *"Everything is a scoped principal"*
(§5), *"Commands go through the gate; reads are SurrealDB-native"* (§7), *"One binary,
edge to cloud"* (§1). It sits directly on top of [`rubix-ext`](../../crates/rubix-ext),
which today supplies only the identity/authorization half.

The starter workspace at `/home/user/code/rust/starter/starter-extensions` already
solved the hard, generic, gate-agnostic parts of this (a process supervisor, a
counter registry, the wire contracts). This document decides **which of those crates
rubix adopts as-is, which it ports, and which it rejects** because they duplicate a
rubix invariant. The guiding rule: reuse the parts that are pure runtime mechanism;
reject the parts that re-implement identity, authorization, or persistence, because
rubix already does those through the gate.

## Where rubix is today

[`rubix-ext`](../../crates/rubix-ext) is a **library, not a managed runtime**. It
models an extension as a scoped `Principal` of kind `Extension` and gives three
faces, all of which terminate in a gated SurrealDB write:

- **provision** — `register_extension` / `grant_extension` create the service-account
  principal and attach capability grants.
- **control** — a JSON-RPC *shape* (`register` / `configure` / `invoke` / `health` /
  `lifecycle`), each mutating verb building a `rubix_gate::Command` and calling
  `apply` ([`control/`](../../crates/rubix-ext/src/control)).
- **data** — `authorize_data_scope` delegates to Zenoh key-space scoping.

The gap is everything that makes an extension *run*:

| Capability | rubix today | starter-extensions |
| --- | --- | --- |
| Identity as scoped principal + capability grants | ✅ `rubix-ext` | ✗ (uses a separate manifest/role model) |
| Gated, audited lifecycle **command** | ✅ writes a `lifecycle` string field through the gate | partial (writes a side row, not gated) |
| A runtime that **acts** on start/stop | ✗ nothing reads the field back | ✅ `starter-ext-supervisor` |
| Live process stats (pid / uptime / rss / cpu / restarts) | ✗ | ✅ `SupervisorHandle::process_stats` |
| Metrics counters (calls, errors, violations, evictions) | ✗ | ✅ `starter-ext-metrics` |
| HTTP admin surface (`GET /extensions`, status, metrics, …) | ✗ | ✅ `starter-ext-server` |
| Wire contracts (`ExtensionId`, `ProcessStats`, flavours) | ✗ | ✅ `starter-ext-spi` |

[`control/lifecycle.rs`](../../crates/rubix-ext/src/control/lifecycle.rs) writes
`{ "lifecycle": "start" }` onto a record and audits it — but **nothing consumes that
field to spawn or kill a process**. `health` ([`control/health.rs`](../../crates/rubix-ext/src/control/health.rs))
is `RETURN true` on the scoped session: it proves the *session* is signed in, not that
any extension is alive. This document fills exactly that gap.

Note: rubix has **no `starter-*` dependency today**. Adopting any starter crate is a
new dependency decision, taken per-crate below.

## What rubix means by "an extension that runs"

The starter system supports three packaging flavours — `builtin` (in-host), `process`
(child binary over stdio JSON-RPC), and `wasm` (WASI-p2 component). rubix only needs
the **`process`** flavour to be real on day one:

- A rubix extension is a **scoped principal** plus, optionally, a **sidecar process**
  that holds that principal's session and does work (ingest, transform, serve a UI
  bundle). The principal is the security boundary; the process is the workload.
- `builtin` collapses to "no process" (the host does the work directly under the
  extension principal) — it reports no process stats, like starter.
- `wasm` is deferred. Nothing in this design precludes it; it is simply not Phase 1.

So the runtime's core job: **given a gated lifecycle record that says `start`, ensure
a supervised child process is running under that extension's scoped session; given
`stop`/`disable`, ensure it is not.** Everything else (stats, metrics, HTTP) is
projection over that.

## Crate-by-crate reuse decision

The starter workspace is ~20 crates. Most are out of scope — the entire
`contributes.*` adapter family (`starter-ext-mcp`, `-grpc`, `-cli`, `-workers`,
`-flow`) surfaces manifest-declared contributions into host subsystems, which is a
**different model** from rubix's "extension is a principal with grants." rubix's
`invoke` + `data` planes already cover *what an extension may do*, gated by capability
grants. We do not adopt the manifest-contributes surface.

| starter crate | Decision | Why |
| --- | --- | --- |
| **`starter-ext-supervisor`** | **Adopt the mechanism** (depend or port `rubix-ext-supervisor`) | The heaviest, most valuable crate and the one rubix wholly lacks: spawn a child under `runtime.bin`, frame stdio JSON-RPC, sample process stats, restart with backoff, process-group kill. Pure OS mechanism, no auth opinion. This *is* start/stop/status. |
| **`starter-ext-metrics`** | **Adopt** (leaf crate) | A `DashMap<ExtensionId, Counters>` of atomics. ~one file, zero policy. Adapters bump it; the admin surface reads it. Trivial to vendor. |
| **`starter-ext-spi`** | **Adopt the shapes** (port into rubix types) | Wire contracts: `ExtensionId`, `ProcessStats`, `ProcessFlavour`, `LifecycleState`. Adopt the *shapes* so the supervisor and projections share a vocabulary, but rebind `ExtensionId` to a rubix `Principal` subject and `LifecycleState` to the `lifecycle` field rubix-ext already writes. |
| **`starter-ext-server`** | **Port the route shapes, re-wire the auth** | The projection logic in [`process.rs`](/home/user/code/rust/starter/starter-extensions/crates/starter-ext-server/src/process.rs), [`metrics.rs`](/home/user/code/rust/starter/starter-extensions/crates/starter-ext-server/src/metrics.rs), `overview.rs`, `events.rs` is reusable. But its **auth** (`with_role(Admin)`) and **persistence** (`EnablementStore` side-row) violate rubix invariants — replace with the gate + `Authenticated` (see below). Mount routes into the existing `rubix-server`, not a parallel Axum app. |
| **`starter-ext-host`** (manifest loader + sealed registry) | **Reject for identity; borrow the registry idea** | rubix's "registry of known extensions" is the set of `Extension` principals in SurrealDB, not a `block.yaml` scan sealed at boot. We keep a lightweight in-memory map of `ExtensionId → SupervisorHandle`, but the source of truth is the DB, not a manifest tree. |
| **`starter-ext-server::EnablementStore` / `starter-ext-store-pg`** | **Reject** | This persists enable/disable as a side DB row, queried at boot — **not through a gate**. rubix already persists lifecycle as a gated, audited record (`rubix-ext` lifecycle command). That record *is* the enablement store. A boot-time reconciler reads it (below). |
| **`starter-ext-sdk` / `-sdk-macros`** | **Defer** | What an extension *author* imports (`#[derive(Extension)]`, `requires!{}`). Relevant once rubix ships an extension SDK; orthogonal to the runtime manager. |
| **`starter-ext-wasm`** | **Defer** | WASI-p2 host. Optional flavour, not Phase 1. |
| **`starter-ext-mcp/-grpc/-cli/-workers/-flow`** | **Reject (model mismatch)** | `contributes.*` adapters. rubix expresses capability through grants + the gate, not manifest contributions. |

**Depend vs. port.** The two repos are separate workspaces with separate SPI types.
For `starter-ext-supervisor` and `starter-ext-metrics` the choice is:

- *Depend* (path/git dep on the starter crates) — fastest, but couples rubix to
  starter's `starter-ext-spi` types and release cadence, and drags `block.yaml`
  assumptions in transitively.
- *Port* into `rubix-ext-supervisor` / `rubix-ext-metrics` under rubix's own SPI —
  more upfront work, but keeps rubix's dependency graph clean and lets the supervisor
  speak rubix `Principal` / `ScopedSession` natively.

**Recommendation: port.** The supervisor is ~one crate of generic process code; owning
it avoids importing the starter manifest model wholesale, and the supervisor needs to
hold a rubix `ScopedSession` (so the child inherits the extension principal's
row-scoped DB access) — which a starter dep cannot give it. Vendor the algorithm,
re-skin the types.

## Cross-cutting: the gate boundary (the rubix invariant the starter lacks)

This is the load-bearing difference and the reason we cannot simply mount
`starter-ext-server`. In starter, enable/disable is a direct store write guarded by an
Admin role. In rubix, **every lifecycle mutation crosses the access gate as a
`Command`** (SCOPE §7), exactly like a record write — and `rubix-ext` already does
this. Consequences we keep:

- Each start/stop/disable produces an **audit record** (principal, namespace, action,
  before/after, correlation id, timestamp). The same path as [ADMIN-API.md](ADMIN-API.md)'s
  principal/grant mutations.
- The gate mints the **correlation id**, so a lifecycle change is traceable end to
  end — including into the supervisor's event log and the metrics it bumps.
- It is **capability-checked fail-closed**: an out-of-grant `lifecycle` call is denied
  before any process is touched (today `DatasourceRegister`; a dedicated
  `extension-manage` capability is the cleaner long-term home — see Open questions).

So the lifecycle record rubix-ext writes is **both** the audit trail **and** the
durable enablement state. There is no separate `EnablementStore`. The supervisor is a
*subscriber* to that state, never an authority over it.

### The bridge: from gated command to running process

Something must turn the gated `lifecycle: start` write into a live child. Two options:

1. **Handler-drives** — the `lifecycle` control handler, after `apply` succeeds,
   calls the supervisor (`supervisor.start(id, session)` / `.shutdown()`), threading
   the gate's correlation id into the supervisor's event ring. Simple, synchronous,
   deterministic; the HTTP response can report the resulting state.
2. **Watcher-drives** — a SurrealDB **live query** on the `lifecycle` field drives the
   supervisor reactively (SCOPE §2, "SurrealDB does as much as possible"). Decouples
   the writer from the runtime and naturally handles writes from any source.

**Recommendation: handler-drives for the transition, plus a boot-time reconciler for
durability.** The live-query path is elegant but makes the HTTP response racy (did the
process actually come up?) and complicates correlation-id propagation. Instead:

- On a `lifecycle` command, after the gated write lands, the handler calls the
  supervisor and reports the observed state.
- On **host boot**, a **reconciler** reads every `Extension` principal's current
  `lifecycle` record and brings the supervisor map to match — extensions last left in
  `start` are re-spawned, `stop`/`disable` stay down. This is rubix's gate-native
  equivalent of starter's "`EnablementStore` queried at boot," and it makes the
  edge-reboot story work (SCOPE §1) without a side table.

```
control::lifecycle(start)
  └─ authorize (capability, fail-closed)
  └─ apply  ──►  gated record { lifecycle: "start" }  +  audit row + correlation_id
  └─ supervisor.start(ext_id, scoped_session, correlation_id)
                         │
                         ▼
              child process under runtime.bin,
              holding the extension's ScopedSession
  ── boot ──► reconciler: read lifecycle records ──► supervisor.start(...) for each "start"
```

## Admin HTTP surface

Mounted into the existing `rubix-server` (it already has `auth.rs`, `http/`, `jobs/`,
`openapi/`), **not** a standalone Axum app. Route shapes ported from
`starter-ext-server`; auth and persistence re-wired through rubix.

| Method & path | Purpose | Source projection |
| --- | --- | --- |
| `GET /extensions` | List every extension: id, version, lifecycle state, restart count | registry map + supervisor handles |
| `GET /extensions/<id>` | Full record: principal, grants, current state, capability-violation counter | rubix-ext principal + grants + supervisor |
| `GET /extensions/<id>/process` | Live pid + sampled process stats (process-flavour, running only) | `SupervisorHandle::process_stats` ([starter process.rs](/home/user/code/rust/starter/starter-extensions/crates/starter-ext-server/src/process.rs)) |
| `GET /extensions/<id>/metrics` | Merged counters + process gauges | `MetricsRegistry::merged` ([starter metrics.rs](/home/user/code/rust/starter/starter-extensions/crates/starter-ext-server/src/metrics.rs)) |
| `GET /extensions/<id>/events` | Paginated event ring; SSE live tail on `Accept: text/event-stream` | supervisor event ring |
| `POST /extensions/<id>/lifecycle` | start / stop / disable | **gated** via `rubix-ext` `lifecycle`, then drives the supervisor |
| `POST /extensions/<id>/health` | Liveness probe | `rubix-ext` `probe_health` + (process-flavour) supervisor liveness |

**Authorization.** Re-wired exactly like [ADMIN-API.md](ADMIN-API.md):

- Reads/mutations use the `Authenticated` extractor
  ([`crates/rubix-server/src/auth.rs:49`](../../crates/rubix-server/src/auth.rs)) — no
  parallel middleware, no starter `with_role`.
- Mutations (`/lifecycle`) cross the gate inside `rubix-ext`, so the capability check
  is the gate's, fail-closed — the same `may_administer` discipline as grant authority
  ([`crates/rubix-gate/src/capability/grant/authority.rs:26`](../../crates/rubix-gate/src/capability/grant/authority.rs#L26)).
- All endpoints are scoped to `auth.principal.namespace`, so a tenant admin only ever
  sees and drives its own namespace's extensions — per-tenant by construction.

Reads are SurrealDB-native (SCOPE §7): `GET /extensions*` run on the caller's scoped
session and are filtered by row-level permissions; process/metrics gauges are read off
the in-memory supervisor/metrics handles (no DB).

## Status & metrics projection

Adopt starter's projection wholesale — it is pure read-side folding with no policy:

- **Process gauges** off the live `SupervisorHandle`: `ProcessStats` (pid, started_at,
  uptime, best-effort rss/cpu), `lifecycle_state`, `restarts_total`,
  `capability_violations_total`, `events_dropped_total`, `group_kills_total`.
- **Adapter counters** off the shared `MetricsRegistry` (calls/errors, ingest
  runs/failures) bumped wherever the extension principal crosses the gate.
- **Graceful degradation**: builtin/wasm/stopped/never-spawned extensions return a
  metrics document with `process: null` and zero gauges — the counters stay
  meaningful. Unknown id → `404`. (Exactly starter's [metrics.rs](/home/user/code/rust/starter/starter-extensions/crates/starter-ext-server/src/metrics.rs)
  behaviour.)

The one rubix-specific add: **capability-violation** and **denied-command** counts can
be fed from the gate's existing fail-closed denials for that principal, so the metrics
view shows authorization health, not just process health.

## Phasing

1. **Supervisor port** — `rubix-ext-supervisor`: spawn/stop/restart a `process`-flavour
   child holding the extension's `ScopedSession`; sample `ProcessStats`; event ring.
   No HTTP yet; drive it from a test.
2. **Bridge + reconciler** — wire `rubix-ext` `lifecycle` to the supervisor
   (handler-drives) and add the boot-time reconciler over lifecycle records. Real
   start/stop now works and survives reboot.
3. **Metrics** — `rubix-ext-metrics` leaf crate; bump it from the gate path and the
   supervisor.
4. **Admin HTTP** — mount `/extensions*` into `rubix-server`, projections ported from
   `starter-ext-server`, auth via `Authenticated` + gate.
5. **Real health** — replace the `RETURN true` session ping with a supervisor liveness
   probe for process-flavour extensions (session ping stays the fallback for builtin).
6. **Later** — extension SDK (`#[derive]`), wasm flavour, UI-bundle serving.

## Open questions

1. **Dedicated capability.** Lifecycle currently reuses `DatasourceRegister`. A
   first-class `extension-manage` (or `extension-lifecycle`) capability is cleaner and
   lets an operator grant "may start/stop extensions" without granting datasource
   registration. Needs a `Capability` variant + grant-profile entry. (See the local
   `rubix-gate/src/capability/kind.rs` work-in-progress in the current diff.)
2. **Sidecar identity handoff.** How does the spawned child obtain its scoped session
   token securely? Options: pass a short-lived gate-minted token over the stdio
   handshake, or have the child authenticate as its principal at startup. Affects how
   `process_stats` and data-plane scoping bind.
3. **Where does `runtime.bin` come from?** rubix has no `block.yaml` manifest. Either
   add a minimal manifest record per extension principal (path, version, flavour), or
   carry it on the principal/config record `rubix-ext` `register` already writes.
   Leaning toward the latter — keep it in the gated config record, no new file model.
4. **Live-query option revisited.** If a future requirement needs lifecycle to react
   to writes from outside the HTTP handler (e.g. a sync from cloud flips a flag),
   promote the watcher-drives path from "rejected" to "added alongside" — the
   reconciler is already 90% of that machinery.

## Authority

- [SCOPE.md](../SCOPE.md) §1 (one binary), §5 (everything is a scoped principal),
  §7 (commands go through the gate).
- [`rubix/docs/sessions/WS-13.md`](../sessions/WS-13.md) — extensions-as-principals
  contracts #1 (gated/audited identically to a user) and #2 (out-of-grant denied
  before effect).
- [ADMIN-API.md](ADMIN-API.md) — the gate-boundary and `Authenticated`/per-namespace
  authorization pattern this surface mirrors.
- Reference implementation to port from: `/home/user/code/rust/starter/starter-extensions`
  (`starter-ext-supervisor`, `starter-ext-metrics`, `starter-ext-spi`,
  `starter-ext-server`).
