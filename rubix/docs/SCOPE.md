# SCOPE — Edge-to-Cloud Data Processing Platform on SurrealDB

Scope for a generic, AI-ready, edge-to-cloud data processing platform: ingest data
from any source, transform and pre-process it in flight, run rules and emit
insights, and serve unified dashboards — the same binary on a single edge device or
in a multi-tenant cloud. The domain is **not** baked in (no equipment / site / point
schema); structure comes from tagging on a graph. SurrealDB is the single store and
brain; everything else attaches around it.

## Principles

1. **One binary, edge to cloud.** The same build runs on a Pi, a Windows box, or in
   the cloud. The only difference is configuration (single namespace vs.
   namespace-per-tenant, sync on/off, cloud-only add-ons).
2. **SurrealDB does as much as possible.** Document, graph, vector, time-series,
   and geospatial in one engine; auth, tenancy, live queries, and the extension
   system are SurrealDB's, not re-implemented. SurrealQL first.
3. **Rules fire offline.** The rule/insight runtime is embedded and runs with no
   cloud dependency.
4. **Generic, not domain-specific.** Tagging on the graph replaces a fixed schema.
   The platform processes *data + tags + rules + datasources*, not a fixed ontology.
5. **Everything is a scoped principal.** Users and extensions authenticate and are
   authorized the same way. An extension can reach the whole stack but only the data
   its grants allow — like a user.
6. **AI-ready by construction.** Vectors live beside the same data the rules and
   dashboards use, so semantic search and agent memory need no separate store.
7. **Commands go through the gate; reads are SurrealDB-native.** Every *mutation/
   command* crosses the access gate (which captures audit/undo and mints the
   correlation id). Reads — including live-query subscriptions — run on a
   gate-issued **scoped SurrealDB session** and are enforced by SurrealDB's own
   row-level permissions, not proxied per message. This is the split that makes
   audit/undo/trace cheap without taxing every read (see Access & policy gate).

## Stack

| Layer | Tech | Role |
| --- | --- | --- |
| Frontend | React · Vite · TanStack · Tailwind-V4 · shadcn/ui · Module Federation | Dashboards; extension UIs load via federation |
| Realtime | SurrealDB live queries (WebSocket) | Push data and insights to apps/dashboards |
| Access gate | SurrealDB auth + grants | One authorize/scope path for every principal |
| Audit | append-only SurrealDB table | Immutable who-did-what, captured at the gate |
| Undo/redo | reversible change records | Revert user-facing definitions; applied through the gate |
| Tracing | correlated spans on the bus | How data/decisions flowed; "why did this rule fire" |
| Rules / insights | Rhai (embedded) | Deterministic, composable, offline-capable |
| Query / compute | DataFusion | Unify datasources into one query surface; vectorized aggregation |
| Datasources | Pluggable connectors (SurrealDB native, Postgres, MQTT, REST, …) | Grafana-style "add a datasource"; each is a DataFusion `TableProvider` |
| Store / brain | SurrealDB | Document + graph(tags) + vector(AI) + time-series + geospatial; auth; SurrealQL; extensions |
| Ingestion / transport | Zenoh | Stream data in for pre-processing; edge↔cloud transport |
| Internal events | tokio channels | In-process component decoupling |
| Preferences | units (metric/imperial) + datetime | Per-user display formatting |

## Architecture

```
╔══════════════════════════════════════════════════════════════════════════════╗
║                              FRONTEND  (web, edge+cloud)                        ║
║   React · Vite · TanStack · Tailwind · shadcn/ui                                ║
║   + Module Federation  ◄── extension UIs load here                             ║
╚════════════════════════════════════╤═════════════════════════════════════════╝
                 WebSocket (live queries) │ + JSON-RPC (control)
                                          ▼
╔══════════════════════════════════════════════════════════════════════════════╗
║                    ACCESS & POLICY GATE   (one gate, every principal)           ║
║   authenticate → authorize → scope.   Backed by SurrealDB auth:                 ║
║   namespace (tenant) · team · user/extension identity · grants · row-level perms║
║   ── users AND extensions pass here for every COMMAND (reads = scoped session) ──║
╚════════════════════════════════════╤═════════════════════════════════════════╝
                                      ▼
        ┌─────────────────────  EVENT BUS  (the spine) ─────────────────────┐
        │  permission-filtered per principal — only see/act on granted data  │
        │   • in-process .......... tokio channels      (internal control)   │
        │   • data-change ......... SurrealDB live queries (record events)   │
        │   • stream/transport .... Zenoh        (ingest + edge↔cloud)       │
        └───┬──────────┬───────────┬──────────────┬───────────────┬─────────┘
            ▼          ▼           ▼              ▼               ▼
     ┌──────────┐ ┌─────────┐ ┌──────────┐ ┌────────────┐ ┌──────────┐
     │  RHAI    │ │DATAFUSION│ │DATASOURCES│ │ INGEST /  │ │  PREFS   │
     │ rules /  │ │ query +  │ │ SurrealDB │ │ PREPROCESS│ │ units ·  │
     │ insights │ │ compute  │ │ Postgres  │ │ (Zenoh in)│ │ datetime │
     └────┬─────┘ └────┬─────┘ │ MQTT · …  │ └─────┬─────┘ └──────────┘
          │            │       └─────┬─────┘       │
          └────────────┴─────────────┴─────────────┘
                                ▼
╔══════════════════════════════════════════════════════════════════════════════╗
║                                SURREALDB CORE                                   ║
║  document · GRAPH(tags) · VECTOR(AI) · time-series · geospatial                 ║
║  auth(teams/users/ext) │ namespaces(tenant) │ live queries │ SurrealQL │ exts   ║
╚══════════════════════════════════════════════════════════════════════════════╝

   ┌──────────────────────────── EXTENSIONS ────────────────────────────────┐
   │  each = a SCOPED PRINCIPAL (service account in SurrealDB auth)           │
   │     control plane → JSON-RPC      data plane → Zenoh                     │
   │     taps EVENT BUS + calls capabilities  ── always via the ACCESS GATE ──│
   │  may reach: rules · query · datasources · store · ingest · events        │
   │  limited to: its granted namespace + row-level perms  (user-like)        │
   └─────────────────────────────────────────────────────────────────────────┘

        EDGE  ══════════  Zenoh / SurrealDB sync  ══════════  CLOUD
   (single namespace,                                  (namespace-per-tenant,
    rules fire offline)                                 cloud-only add-ons)
```

## Components

### SurrealDB core

The single store and brain. Holds all native data as document records, with
**graph edges for tagging** (`record -> tagged -> tag`) instead of a fixed schema,
**vectors** for embeddings/semantic search, **time-series** for history, and
**geospatial** where needed. Owns auth, tenancy (namespaces), live queries
(the realtime/WebSocket surface), and the extension system. SurrealQL is the
primary query language; DataFusion sits above only where cross-datasource
unification or heavy vectorized aggregation is wanted.

### Access & policy gate

One identity, **two enforcement points** — this is the load-bearing distinction, so
it is stated precisely rather than collapsed into "one gate does everything":

- **Commands (mutations) → app gate.** Every write/command goes through the gate.
  The gate authenticates the principal, checks **capability grants**, captures the
  before/after for audit+undo, mints the correlation id, and only then applies the
  change. Clients never write directly to SurrealDB.
- **Reads (incl. live queries) → scoped SurrealDB session.** At auth time the gate
  issues a **scoped session token**; SurrealDB's own **row-level permissions** then
  enforce what that session may read, natively, with no per-message app proxy. This
  is why native live queries and the "one gate audits everything" goals do not
  conflict — reads are not gated per message, they are scoped once at the session.

#### Two authz layers (do not conflate)

| Authz | Governs | Enforced by |
| --- | --- | --- |
| **Data-record perms** | which SurrealDB records a principal may read/write | SurrealDB row-level permissions (native), via the scoped session |
| **Capability grants** | non-record actions: register a datasource, invoke a rule, publish ingest, query an external (Postgres/MQTT) datasource, subscribe to a Zenoh key-space | **app-enforced** by the gate |

SurrealDB's permission engine only governs SurrealDB data. Everything that touches
another plane (DataFusion over external sources, Zenoh streams, rule invocation,
extension registration) is an **application capability** the gate enforces. Both
layers key off the *same* principal identity — so it is one identity model, but
honestly two enforcement layers, not one.

For Zenoh, the gate resolves the principal's permitted **key-space once at
subscription setup** (a capability decision), not per message — so high-throughput
streams stay un-taxed while still being scoped.

### Event bus

Three planes, presented to principals as one permission-filtered eventing surface:

- **In-process** — tokio channels for component-to-component control events inside
  the binary. No serialization, no network.
- **Data-change** — SurrealDB live queries are the pub/sub for "a record
  appeared/changed" (new reading, new insight, new firing). This is also the
  frontend's realtime feed; extensions subscribe to the same mechanism.
- **Stream / transport** — Zenoh for high-throughput data in (pre-processing) and
  edge↔cloud movement.

Subscription scope is set **once**, not per message: live-query reads are filtered
by SurrealDB row-level permissions on the principal's scoped session; a Zenoh
key-space is resolved by the gate at subscribe time. A principal sees all event
*types* it is granted, but only the *records* its perms allow — enforced by the
plane that owns the data, not by a per-message app proxy.

### DataFusion — query and compute

The unification layer over datasources. Each datasource registers as a
`TableProvider`; the dashboard and rules query a single surface that spans
SurrealDB plus any external sources. DataFusion also provides the vectorized
compute for rule aggregations (time-window rollups feeding a rule decision).
Heavy aggregation belongs here, not in Rhai.

### Datasources

The Grafana model: a datasource is a declared, pluggable connection that a
dashboard or rule can read in a unified way. SurrealDB is the default/native
datasource; Postgres, MQTT, REST, and others attach as connectors. "Unlimited
datasources" means adding a connector, not changing the core. Datasources are a
kind of extension.

### Rhai — rules and insights

The embedded, deterministic rule/insight runtime. Rules are composable (rules call
rules), fire offline, and consume values computed by DataFusion (the time-window
math) to produce a decision/insight that is recorded back to SurrealDB and
published as a data-change event. Rhai owns the *decision*; DataFusion owns the
*data*.

### Ingestion and pre-processing

Sources and ingestion extensions publish to Zenoh; pre-processing nodes consume
in flight (decimate, filter, enrich) and then persist to SurrealDB. Raw high-rate
streams are processed before persistence rather than written first and queried
back.

### Extensions

First-class, out-of-process participants that extend any part of the stack:
datasource connectors, ingestion/transport adapters, processing nodes, rule packs,
sinks (notify/write-back), frontend panels, and event subscribers. Two planes:

- **Control → JSON-RPC** — register, configure, invoke, health, lifecycle.
- **Data → Zenoh** — high-throughput streaming in/out (not one JSON-RPC call per
  sample).

See **Extensions as principals** below for the access model.

### Frontend

React + Vite + TanStack + Tailwind + shadcn/ui. Dashboards render any datasource
through one path. Extension UIs load via **module federation**. Realtime comes from
SurrealDB live queries over WebSocket; control actions go over JSON-RPC. Display
respects per-user **unit (metric/imperial)** and **datetime** preferences.

## Extensions as principals

The load-bearing access rule: an extension is modeled as a **service account in
SurrealDB auth**, scoped to a namespace — the *same identity model* as a user.
Enforcement, though, is the two layers above: SurrealDB row-perms for data records,
app-enforced capability grants for cross-plane actions. Consequences:

- **One identity, two enforcement layers.** No separate "plugin trust" path — users
  and extensions are the same kind of principal — but data-record perms are
  SurrealDB-native while capability grants (ingest, rule invoke, datasource
  register, Zenoh key-space) are app-enforced. Same identity, two enforcers.
- **Reach everything, restricted to a lane.** An extension may touch every plane
  (events, query, datasources, store, ingest) but only within its granted data.
  "Access the whole bus" means all event *types*, not all *records*.
- **Capabilities are grants.** What an extension may *do* — publish ingest, write
  insights, invoke a rule, register a datasource — is a grant on its principal. A
  read-only extension, an ingest-only extension, and a tenant-admin extension all
  use the same mechanism.
- **Edge inherits it.** On a single-namespace edge node the gate resolves to the
  one tenant automatically; extensions and rules keep working offline because the
  gate, bus, and SurrealDB are all local.

## Cross-cutting: audit, undo/redo, tracing

These three are designed in at the two chokepoints every action already crosses —
the **access gate** and the **event bus** — so they are byproducts of routing, not
three bolted-on systems. They are distinct in mutability, retention, and purpose and
must not be conflated:

| Concern | Answers | Mutability | Retention |
| --- | --- | --- | --- |
| Audit | who did what, when | append-only, immutable | long / compliance |
| Undo/redo | revert my last change | mutable stack (popped) | short / session-scoped |
| Tracing | how data/decisions flowed | append-only, bounded | rolling / sampled |

```
              ┌──────────── ACCESS GATE ────────────┐      ┌──── EVENT BUS ────┐
   mutation ─►│ capture change  ──► AUDIT (immutable)│  work │ emit span ─► TRACE│
              │                 └─► UNDO  (inverse)   │ ─────►│ (per principal)  │
              └──────────────────────────────────────┘      └──────────────────┘
                          one correlation id stamped on all three
```

### Correlation id (the linchpin)

A trace id is minted at the gate (principal actions) or at ingest (data), carried
on every bus event, and stamped into audit records, undo change records, and spans.
It is the single thread that lets you pivot: an insight → its rule-run trace → the
audit of any action it triggered → the undo entry that changed the rule.

### Audit log

- Captured at the **access gate** on every mutating action (and opt-in sensitive
  reads). Because all principals — users and extensions — cross the gate, audit is
  uniform and automatic; an extension is audited identically to a user.
- Record: principal, namespace, action, target record, before/after summary,
  correlation id, timestamp. The before-image is taken atomically with the write
  (SurrealDB `RETURN BEFORE`), so capturing it does not add a separate
  read-before-write round trip.
- Stored append-only in SurrealDB per namespace; immutability enforced by SurrealDB
  permissions (no UPDATE/DELETE grant to any principal but the system). Synced
  edge→cloud.
- Never popped, never undone — the immutable truth, separate from the undo stack.

### Undo/redo

- Mutations to **user-facing definitions** (dashboards, rules, tags, datasource
  config) produce a reversible change record — a forward plus an inverse, cheap via
  SurrealDB document before/after snapshots.
- The undo stack is scoped per principal + resource. Undo applies the inverse
  **through the gate**, so it is permission-checked and itself audited.
- Boundary: undo covers definitions/config only — never the data plane (readings,
  insight firings) and never the audit log.
- Audit and undo derive from the **same captured change**: audit takes the immutable
  projection, undo takes the mutable stack. One capture, two consumers.

### Tracing

- Spans emitted to the **event bus** as work flows ingest → pre-process → rule →
  insight → sink, correlated by trace id.
- The Rhai engine emits a **span tree per rule evaluation** (which sub-rules ran,
  the values seen, the decision) — the deterministic answer to "why did this fire".
- Stored in SurrealDB with bounded/rolling retention and sampling (high volume — not
  kept forever).
- Rendered as a waterfall; an existing Apache-2.0 trace-view frontend fits directly.

Edge note: all three capture locally and sync up, and work offline because the gate,
bus, and SurrealDB are local.

## Edge and cloud profiles

The same binary, configured two ways:

- **Edge** — single SurrealDB namespace, rules fire offline, local dashboard, sync
  to cloud over Zenoh when connected. No multi-tenancy code path; the gate resolves
  to the one tenant.
- **Cloud** — namespace-per-tenant for isolation, teams/users via SurrealDB auth,
  fleet-wide views, and cloud-only add-ons.

### Sync and conflict model

Same engine on both sides removes the *schema-translation* problem but **does not**
solve reconciliation. SurrealDB has no mature multi-master replication (live queries
are change notification, not replication), so sync is an **application-level shipper
over Zenoh**, not DB replication — and it needs an explicit conflict model:

- **Data plane (readings, insights, audit, traces) — append-only, edge-owned.**
  Each edge writes into a **partition keyed by its own device/edge identity**, so
  two edges never write the same records. Reconciliation is ordering + dedup by id,
  not merge. No multi-master conflict by construction.
- **Config plane (dashboards, rules, tags, datasource defs) — the real conflict
  surface**, since the same definition can be edited in cloud and on edge. Default
  policy: **ownership** — cloud owns shared/tenant config, edge owns local-only
  config — with **last-write-wins + the audit log as tiebreak** where overlap is
  unavoidable. A CRDT model is only warranted if genuine concurrent edit of the same
  definition becomes a requirement.

This keeps the hard case (config) small and the high-volume case (data) conflict-free
by partitioning. The unresolved edges are tracked in Open Questions.

## Scale path

SurrealDB is the historian by default and is sufficient for edge and moderate cloud
volumes. The first scale lever for heavy time-window rollups is **pre-aggregated
rollup tables** written in SurrealDB as data lands. An external time-series engine
(e.g. TimescaleDB) is an **optional** datasource behind the DataFusion seam, added
only when pre-aggregation is not enough — it is never required for the core to run.
All history access stays behind a single historian boundary so this swap is
contained, not a refactor.

**Known bet — single-engine concentration.** Pub/sub fan-out, audit, high-volume
traces, time-series, vector, and auth all land on one SurrealDB instance. The
DataFusion + optional-Timescale seam is a **read**-scale escape hatch; there is no
equivalent **write/pub-sub** hatch today. This is a deliberate bet that
concentration buys simplicity now; if it bites, the likely relief valves are moving
traces/audit to a separate store and offloading live-query fan-out to Zenoh. Tracked
in Open Questions.

## Non-goals

- No fixed domain schema (equipment / site / point). Structure is tagging on the
  graph.
- No second store for vectors, graph, or time-series; SurrealDB covers them.
- No required external database. Postgres/Timescale are optional datasources.
- No privileged extension trust path; extensions are scoped principals.
- No multi-tenancy on edge (single namespace by design).
- No JSON-RPC for high-throughput data; that is Zenoh's plane.

## Open questions

1. **Edge↔cloud sync / conflict resolution (highest risk).** The shipper-over-Zenoh
   model and the partition+ownership conflict policy above are a starting position,
   not a solved design. Open: ordering/dedup guarantees under long edge offline
   windows; how config-plane ownership boundaries are declared and enforced; whether
   LWW-with-audit-tiebreak is acceptable or a CRDT is needed for any shared
   definition; replay/idempotency on reconnect. SurrealDB has no mature multi-master
   replication, so this is app-level and must be designed, not assumed.
2. **Single-engine write/pub-sub concentration.** No escape hatch today for write
   and live-query-fan-out load (only reads have the DataFusion/Timescale seam).
   Decide trigger points and relief valves (separate trace/audit store; Zenoh
   fan-out offload) before the concentration bites.
3. **SurrealDB time-series at target volume** — where pre-aggregated rollups stop
   being enough and an external historian datasource is warranted (verify at real
   cardinality).
4. **SurrealDB storage engine on edge** — embedded backend choice (durability/ops),
   given SurrealKV maturity.
5. **Rhai rule vocabulary** — confirm time-windowed conditions and hysteresis/
   debounce ("for N minutes") split cleanly: DataFusion computes the window, Rhai
   makes the decision.
6. **Live-query fan-out limits** — scale ceiling of SurrealDB live queries as the
   data-change bus under many subscribers; where Zenoh takes over.
7. **Extension scope enforcement on the bus** — how the access gate filters a
   Zenoh data subscription by the extension's grants without a per-message gate
   cost.
8. **Reusable cloud-only insight/validation surface** — whether to harvest an
   existing Apache-2.0 eval/labeling frontend + schema for the cloud "prove a rule
   before deploy" loop, or build it native.
9. **Undo collaboration model** — per-user linear undo per resource vs. shared/
   collaborative undo, given the audit log is the global truth either way.
10. **Trace retention and sampling** — what to keep, for how long, and the sampling
   rate, given rule-run and data-flow spans are high volume.
