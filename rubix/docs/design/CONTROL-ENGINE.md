# Control Engine — Sidecar Integration

Design for integrating [`control-engine`](../../../bin/control-engine) — a C++20
graph-dataflow execution engine — into rubix as an **out-of-process sidecar**, on both
the edge and cloud profiles. Reads against [SCOPE.md](../SCOPE.md): *"Everything is a
scoped principal"* (§5), *"Commands go through the gate; reads are SurrealDB-native"*
(§7), *"One binary, edge to cloud"* (§1). It sits on top of the runtime work in
[EXTENSION-RUNTIME.md](EXTENSION-RUNTIME.md) — control-engine is the first real
`process`-flavour extension that runtime supervises.

## What control-engine is

It is **not a library**; it is a multi-process system, and that fact decides the whole
integration:

- A long-lived engine process owns Boost.Interprocess **shared memory** (engine writes,
  everyone else maps it read-only), runs a continuous duty-cycle graph-evaluation loop,
  and persists to its **own SQLite database**.
- It **spawns its own extension processes** and speaks **Cap'n Proto RPC over a Unix
  domain socket** (`unix:/tmp/control-engine.sock`, override via
  `CE_PJNUBE_CORE_ENGINE_ADDRESS`).
- Its #1 design principle is *"an extension can never crash or hang the engine"* — which
  holds **only** because extensions are separate processes behind a fault boundary.

The engine models a directed graph of **components** (nodes with typed `input` / `output`
/ `config` properties) connected by **edges** (dataflow + evaluation-order dependency).
It evaluates in dependency order, supports property **overrides** with TTL, **soft-delete +
undo**, and a **Change-of-Value (COV)** event stream. That is squarely a control /
automation runtime — the kind of thing a rubix datasource or rule pack would drive.

## The decision: sidecar service, not Rust bindings

We run control-engine as a **supervised sidecar process** and talk to it over its
network boundary. We do **not** link `libcontrol_engine` into the rubix binary via FFI.

| Option | Verdict | Why |
| --- | --- | --- |
| **Rust FFI bindings** (link the engine in-process) | **Reject** | The engine *is* `main()`: it forks child processes, installs its own SIGINT/SIGTERM handlers, and runs a `kj::WaitScope` event loop. Hosting that inside a tokio runtime means two event loops and two signal regimes in one address space. It also discards the process fault boundary that is the engine's central invariant. |
| **Read-only SHM from Rust** (mmap the pools directly) | **Reject** | Would mean reimplementing the pool / arena / seqlock memory layout in Rust against an ABI the engine's own docs describe as actively churning ("SDK lockdown"). Worst effort-to-benefit of the three, and brittle on every engine bump. |
| **Sidecar service over RPC** | **Adopt** | The engine already ships a network boundary (Cap'n Proto RPC; plus a NATS CRUD/COV surface via its `ce-nats` extension). control-engine becomes exactly what SCOPE calls an extension/datasource **principal**: out-of-process, control-plane over RPC, scoped by grants at the gate. No C++ changes required to start. |

This also drops cleanly into the existing model: in
[EXTENSION-RUNTIME.md](EXTENSION-RUNTIME.md) terms, control-engine is a **`process`-flavour
extension** — a scoped `Principal` of kind `Extension` whose workload is the engine
binary, supervised by `rubix-ext` runtime (spawn under `runtime.bin`, restart with
backoff, process-group kill, process stats). The engine's socket is its control plane.

## Transport: native Cap'n Proto, behind one Rust trait

This integration needs **two surfaces, not one** — and that is the single most important
correction to make up front. The engine's native Cap'n Proto RPC is a **command surface
only**: a `grep` of [`app.capnp`](../../../bin/control-engine/src/capnp/app.capnp) /
[`core.capnp`](../../../bin/control-engine/src/capnp/core.capnp) for `subscribe` / `cov` /
`tree` / `event` returns **nothing**, and the `Component` capability it hands back is
write-oriented (update / override / status — no children walk, not even a property-value
read). Reads happen by **mapping SHM**; the change stream (COV) is exposed by the
**`ce-nats` extension** (`get` with `depth`/`nested`/`withEdges` for tree snapshots; `sub`
+ `pub` for property/edge/tree/metadata COV), not by the RPC. So:

| Surface | What it carries | Transport |
| --- | --- | --- |
| **Commands** | add / remove / patch / override / edge / bulk / restore | **Native Cap'n Proto RPC** (`App`/`Component`/`Edge`). |
| **Reads + COV** | tree snapshots, property values, change subscriptions | **`ce-nats` today**; a **first-class engine read+event RPC API** is the clean long-term target (new upstream C++ work) — or, last resort, a SHM reader in Rust (rejected for the same ABI-churn reason as §"The decision"). |

The non-negotiable design rule that makes both safe to commit to:

> **Callers in rubix only ever see a narrow Rust trait. The wire format(s) live behind it
> and are replaceable.**

```rust
/// The rubix-side contract for a control-engine instance.
/// Command methods → native capnp. Read/observe methods → ce-nats (today).
#[async_trait]
pub trait ControlEngine: Send + Sync {
    // commands — native Cap'n Proto (App / Component / Edge)
    async fn add_node(&self, parent: NodeRef, spec: NewNode) -> Result<NodeId>;
    async fn patch(&self, node: NodeId, props: Vec<PropPatch>) -> Result<Node>;
    async fn set_override(&self, node: NodeId, prop: &str, v: Value, ttl: Duration) -> Result<()>;
    async fn clear_override(&self, node: NodeId, prop: &str) -> Result<()>;
    async fn add_edge(&self, edge: EdgeSpec) -> Result<EdgeId>;
    async fn remove_node(&self, node: NodeId) -> Result<DeletedItems>; // returns UIDs for undo

    // reads / observe — NOT in native RPC; ce-nats (or a future engine read API)
    async fn get_tree(&self, root: NodeRef, depth: i32) -> Result<Tree>;
    async fn subscribe_cov(&self, scope: CovScope) -> Result<CovStream>;
}
```

The trait — not the wire format — is the long-term investment. The command and read halves
can move independently behind it: if the engine later grows a first-class read+event RPC,
the read methods re-point with no caller change; if native capnp ever proves impractical
for a remote instance, commands fall back to `ce-nats` too.

### Why native (for commands) is acceptable cost

The engine's docs are explicit that the **RPC boundary is not a hot path** ("RPC
round-trip dominates; most callbacks fire at human rate"); the hot path is internal graph
eval, which rubix never touches. So we pay codegen maintenance, not latency:

- Generate Rust stubs from `app.capnp` / `core.capnp` (the `capnpc` crate / `capnp`
  runtime). **Pin to a committed schema revision**; regenerate deliberately on engine
  bumps, never track HEAD. The schemas are versioned and changing (e.g. `*ByHandle`
  methods removed in v0.10.0).
- Drive the client on a dedicated capnp/`kj`-compatible async task; bridge to tokio with
  a channel so the trait surface is plain `async fn`.

### Obtaining the `App` capability is not free

There is no direct "connect and get an `App`" method. The **only** schema path to `App` is
`Engine.registerExtensionManager`
([`core.capnp:10`](../../../bin/control-engine/src/capnp/core.capnp)), and in the engine
`results.setApp(app_)` runs **inside the loop over the supplied extension definitions**
([`engine.cpp` `registerExtensionManager`](../../../bin/control-engine/src/engine/engine.cpp)).
Register **zero** extensions and the loop body never executes — the caller gets no usable
`App`. Phase 1's "client" therefore must **register a minimal `ExtensionManager` with at
least one extension definition** to obtain the `App` capability, *or* we add a dedicated
app-client method upstream. This is a real task, not a footnote (see Open Questions).

## Client vs. extension — the one thing that sizes the job

The engine's RPC is **bidirectional**. Two interfaces (in
[`core.capnp`](../../../bin/control-engine/src/capnp/core.capnp)) point in opposite
directions, and which one rubix implements decides how big this is:

- **`App` / `Component` / `Edge`** (in `app.capnp`) — *rubix calls the engine*. Create
  nodes, patch properties, set overrides, wire edges, bulk add/remove, restore
  soft-deleted items. This is the **command client** side. (Note: it does **not** include
  reads/COV — those come from the second surface above.)
- **`Extension`** (`startExtension`, `evaluateComponent`, `getAllComponentUpdates`,
  `callAction`, `stopExtension`, …) — *the engine calls you*. This is what an extension
  process implements to provide component logic.

**Phase 1 is command-client + ce-nats reads.** rubix orchestrates and observes an engine
that runs the existing C++ extensions (math, util, ce-nats, …); it does not provide
component logic. So rubix implements the `App`-side command caller, consumes reads/COV via
ce-nats, and registers a minimal extension manager only to obtain `App` (above) — it does
*not* implement the `Extension` server. The lift is "generate stubs + write an async
command client + a ce-nats read client + map types".

**Phase 2 (deferred): rubix as a control-engine extension.** If rubix needs to supply
component behaviour to the graph (e.g. expose a SurrealDB-backed datasource or a Rhai
rule as engine components), it implements the `Extension` interface in Rust. That is a
materially bigger task (it must serve `evaluateComponent` within the 10ms eval budget,
manage the SHM read side, honour the soft-delete callback contract). Flagged here so the
Phase 1 trait is shaped not to preclude it, but it is explicitly out of scope now.

## Mapping to the rubix access model

control-engine commands cross the **gate** like every other principal's commands; reads
ride its COV stream onto the event bus. The two-layer authz from SCOPE §"Two authz
layers" applies cleanly:

| rubix concern | control-engine binding |
| --- | --- |
| **Principal** | One `Extension` principal per engine instance (per namespace/tenant in cloud; the single edge tenant on edge). |
| **Capability grants** (app-enforced) | `control-engine:write` (add/remove/patch/override), `control-engine:read` (tree/COV subscribe), `control-engine:admin` (`EngineConfig`: shutdown / restart / flushDb / purge — see the auth-token caveat in §Admin & security). Mutating trait methods build a `rubix_gate::Command` and go through `apply`, so every engine command is **audited**. |
| **Data-record perms** | Not SurrealDB rows — the engine owns its own SQLite. So engine state is governed entirely by **capability grants at the gate**, not row-level perms. The gate decides *whether* a principal may drive the engine; the engine's own state is opaque to SurrealDB. |
| **Audit / undo** | **Undo is partial and must not be over-claimed.** Deletes are genuinely reversible: the engine returns `DeletedItems { componentUids, edgeUids }` and `restoreItems` revives them within the 24h soft-delete window — the gate's undo stack stores those UIDs and replays the restore. **But `patch` / `setOverride` have no RPC-derivable inverse** — the native command surface cannot read the prior value (reads aren't in the RPC), so there is no before-image to invert to. To make those undoable, the gate must capture **before/after on a SurrealDB shadow record** at command time (reading the prior value via the ce-nats/read surface *before* mutating). That matches SCOPE's undo model (before/after of a definition), and the engine soft-delete window is only the safety net beneath delete-undo, not a general one. |
| **Event bus** | COV subscriptions (property / edge / tree / metadata, via ce-nats) are resolved **once at subscribe time** (a capability decision, like a Zenoh key-space), then flow as data-change events. Matches SCOPE's "subscription scope set once, not per message". |
| **Correlation id** | Minted at the gate on each engine command; carried into the audit/shadow record. The engine has no notion of it, so the linkage lives on the rubix side keyed by the returned UIDs. |

## Edge and cloud topology

Both profiles run an engine instance; the binary is the same, the wiring differs. The
**sharp edge** to respect: control-engine assumes **one SHM namespace + one socket + one
SQLite DB per process**. There is no in-process multi-tenancy. So tenant isolation in
cloud is achieved by **process-per-tenant**, not by one engine serving many namespaces.

Critically, isolation requires **three** per-instance settings, not two — the SHM
filename is the one easy to miss. The engine's default SHM name is `control_engine`
([`defines.h:8`](../../../bin/control-engine/src/defines.h)), read via
`CE_PJNUBE_CORE_SHM_FILENAME` and **unlinked at boot**
([`main.cpp`](../../../bin/control-engine/src/main.cpp)). Two instances sharing the
default would map — and boot-scrub — each other's `/dev/shm` pools. So each instance needs
a unique `CE_PJNUBE_CORE_SHM_FILENAME`, and it must be **inherited by the extension-manager
child processes** (they `shm_open` the same name), exactly like the socket address.

| | Edge | Cloud |
| --- | --- | --- |
| Instances | One engine per device. | **One engine process per tenant namespace.** |
| Socket | Engine-owned runtime dir (see §Admin & security), e.g. `unix:/run/control-engine/edge.sock`. | Per-instance socket, e.g. `unix:/run/control-engine/<tenant>.sock` — supervised by runtime, one `SupervisorHandle` per instance. |
| **SHM name** (`CE_PJNUBE_CORE_SHM_FILENAME`) | `control_engine` (default fine — single instance). | **Per-instance, e.g. `ce_<tenant>`** — inherited by extension-manager children. Missing this corrupts pools across tenants. |
| Data path | Local `CE_PJNUBE_CORE_DATA_PATH`. | Per-tenant data dir; the engine's SQLite is that tenant's control state. |
| Offline | The point of edge — graph fires locally with no rubix/cloud dependency, exactly like SCOPE's "rules fire offline". rubix drives it when connected; the engine keeps running when not. | N/A (always-connected). |
| Sync | Engine results that rubix persists to SurrealDB ride the existing Zenoh shipper edge→cloud, partitioned by edge identity (append-only data plane, no conflict). | Authoritative tenant store. |

The per-tenant-process model fits `rubix-ext` runtime directly: each engine instance is a
supervised child with its own `ExtensionId → SupervisorHandle`, its own scoped session,
its own socket, its own SHM name and data dir. The cost is **N engine processes in
cloud** — acceptable because the engine is lightweight per instance (lazy-paged SHM,
~tens of MB RSS) and the isolation is a feature, not overhead.

## Failure model

The sidecar boundary gives us clean failure semantics for free:

- **Engine crash/hang** → the rubix client's RPC calls fail or time out; runtime's
  supervisor restarts the engine with backoff (the same mechanism that supervises any
  process-flavour extension). rubix surfaces the instance as unhealthy via the
  `/extensions` admin surface. No rubix-side corruption is possible — we never share the
  engine's address space.
- **Engine restart** → the engine replays its own SQLite on boot (Grade-0 persistence;
  soft-deleted rows reserved at original UIDs). rubix must treat the engine as
  authoritative for its own UIDs after a restart and **re-establish COV subscriptions**
  — they do not survive an engine restart.
- **rubix restart** → the engine keeps running (it is a separate process); the client
  reconnects to the existing socket and re-subscribes COV. On edge this is exactly the
  "engine keeps firing while the controller is down" property we want.

## Admin & security

Two engine-side realities need an explicit plan; neither is a blocker, but skipping them
leaves a soft boundary:

- **Don't run the socket in `/tmp`.** The default `unix:/tmp/control-engine.sock` is not a
  production boundary — a world-writable parent dir invites a symlink swap, and the engine
  source itself flags this and recommends moving the socket to an **engine-owned runtime
  dir** ([`server_context.hpp`](../../../bin/control-engine/src/util/server_context.hpp):
  *"move the socket to a runtime directory the engine owns, e.g. `/run/control-engine/`"*).
  So: each instance binds inside a dir rubix/runtime creates and owns (`0700`,
  service-user), and sets `CE_PJNUBE_CORE_LINUX_GROUP` so only the rubix service user can
  connect. That is the OS-level half of the access boundary beneath the gate's capability
  check.
- **`EngineConfig` is guarded by a hard-coded token today.** Access to the privileged
  `EngineConfig` interface (shutdown / restart / flushDb / purge) is gated in the engine by
  a literal `systemExtensionAuthToken_ = "auth-token"`
  ([`engine.cpp`](../../../bin/control-engine/src/engine/engine.cpp)). Mapping
  `control-engine:admin` onto it without changing the engine buys no real protection. So
  **Phase 1–2 avoid `EngineConfig`** (lifecycle is driven by the supervisor killing/starting
  the *process*, not by the RPC); exposing it later requires an upstream change to a real
  secret with a rotation story.

## Phasing

1. **Phase 1 — command client + ce-nats reads.** Generate pinned Rust capnp stubs;
   register a minimal extension manager to obtain `App`; implement the command half of the
   `ControlEngine` trait over `App`/`Component`/`Edge` (add/patch/override/edge/remove/
   restore); implement the read half (`get_tree`, `subscribe_cov`) over **ce-nats**. Single
   edge instance, engine-owned socket dir, no `EngineConfig`. No gate wiring yet — prove
   both wires work.
2. **Phase 2 — gate + runtime integration.** Route mutating trait methods through
   `rubix_gate::Command`/`apply` (audit on every command; before/after **shadow record** in
   SurrealDB for patch/override undo, restore-UID undo for deletes); register the engine as
   an `Extension` principal with `control-engine:*` grants; supervise it via `rubix-ext`
   runtime (one `SupervisorHandle`). COV subscribe becomes a capability decision on the
   event bus.
3. **Phase 3 — cloud multi-instance.** One supervised engine process per tenant
   namespace; per-instance socket + **SHM name** + data dir; health/metrics on
   `/extensions`.
4. **Phase 4 (deferred) — rubix as an engine extension.** Implement the `Extension`
   server interface in Rust to expose SurrealDB datasources / Rhai rules as engine
   components. Only if a real requirement to *provide* component logic appears.
5. **Optional upstream — first-class read+event RPC.** If ce-nats proves an awkward read
   plane (broker dependency, JSON hop), add a native read/subscribe API to the engine and
   re-point the trait's read methods. Pure win behind the trait; sequence by need.

## Open questions

1. **capnp Rust ergonomics under churn.** The schemas are versioned and actively
   changing. Confirm the regen workflow (vendored generated code vs. `build.rs` codegen)
   and how we gate a rubix release against an engine schema revision. A schema mismatch
   must fail loudly at startup, not silently mis-decode.
2. **COV delivery semantics.** The engine's COV is best-effort change notification, not a
   durable log. Do we need replay/gap-detection on reconnect (engine restart loses
   subscriptions), or is a full `get_tree` resync on reconnect sufficient? Likely the
   latter for Phase 1.
3. **UID ↔ rubix identity.** Engine UIDs are `uint32_t`, per-instance, and reset
   generations on engine restart. Decide the stable rubix-side identity for an engine
   node (e.g. `(instance_id, path)` vs. raw UID) so undo and audit references survive an
   engine restart.
4. **Cloud process density.** Per-tenant engine processes — confirm the RSS/socket/fd
   budget at the target tenant count, and whether idle tenants should have their engine
   stopped (and cold-started on first command) to cap resident processes.
5. **Bidirectional RPC on the rubix side.** Even command-only, the `App` interface returns
   capabilities (`Component`, `Edge` objects) and the engine may push via the registered
   `ExtensionManager`. Confirm the chosen Rust capnp stack drives this two-way object
   model cleanly on a tokio bridge.
6. **Obtaining `App` without owning extensions.** The only path to `App` is
   `registerExtensionManager`, and the engine sets the `App` result *inside* the loop over
   supplied extension defs — zero extensions likely yields no `App`. Decide: register a
   benign placeholder extension manager (and what it claims), or land an upstream
   `Engine.getApp()` method. This gates Phase 1.
7. **Read plane: ce-nats vs. native.** ce-nats adds a broker + JSON hop but is the *only*
   complete read/COV surface today. Confirm a broker is acceptable on edge (it implies
   running NATS alongside the engine), or prioritise the optional upstream read RPC
   (Phase 5) sooner. Don't assume native RPC can read — it can't.
8. **Shadow-record cost for undo.** Capturing before/after for every patch/override means a
   read (via ce-nats) *before* each mutating command — an extra round trip on the command
   path. Confirm that's acceptable, or scope undo to deletes-only initially.
