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
7. **Audit, undo, and tracing are byproducts of routing.** Everything crosses the
   access gate and the event bus, so who-did-what (audit), what-changed (undo), and
   how-it-flowed (trace) are captured at those two chokepoints — not bolted on.

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
║   ── users AND extensions pass through here for EVERY action ──                 ║
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

A single authorize-and-scope path that **every** principal passes through — users
and extensions alike. It authenticates against SurrealDB auth, resolves the
principal's namespace (tenant), team, capability grants, and row-level
permissions, and stamps that scope onto every downstream action: event
subscriptions, datasource queries, store reads/writes, rule invocations, and Zenoh
streams. There is no privileged bypass; an extension is a non-human user.

### Event bus

Three planes, presented to principals as one permission-filtered eventing surface:

- **In-process** — tokio channels for component-to-component control events inside
  the binary. No serialization, no network.
- **Data-change** — SurrealDB live queries are the pub/sub for "a record
  appeared/changed" (new reading, new insight, new firing). This is also the
  frontend's realtime feed; extensions subscribe to the same mechanism.
- **Stream / transport** — Zenoh for high-throughput data in (pre-processing) and
  edge↔cloud movement.

Subscriptions are filtered by the access gate: a principal sees all event *types*
it is granted, but only the *records* its grants allow.

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
SurrealDB auth**, scoped to a namespace with capability grants and row-level
permissions — identical machinery to a user. Consequences:

- **One identity model.** No separate "plugin trust" path; one auth system for
  users and extensions.
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
  correlation id, timestamp.
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
  fleet-wide views, and cloud-only add-ons. Because both sides use SurrealDB as the
  historian, edge↔cloud sync is homogeneous (SurrealDB→SurrealDB) rather than a
  cross-engine bridge.

## Scale path

SurrealDB is the historian by default and is sufficient for edge and moderate cloud
volumes. The first scale lever for heavy time-window rollups is **pre-aggregated
rollup tables** written in SurrealDB as data lands. An external time-series engine
(e.g. TimescaleDB) is an **optional** datasource behind the DataFusion seam, added
only when pre-aggregation is not enough — it is never required for the core to run.
All history access stays behind a single historian boundary so this swap is
contained, not a refactor.

## Non-goals

- No fixed domain schema (equipment / site / point). Structure is tagging on the
  graph.
- No second store for vectors, graph, or time-series; SurrealDB covers them.
- No required external database. Postgres/Timescale are optional datasources.
- No privileged extension trust path; extensions are scoped principals.
- No multi-tenancy on edge (single namespace by design).
- No JSON-RPC for high-throughput data; that is Zenoh's plane.

## Open questions

1. **SurrealDB time-series at target volume** — where pre-aggregated rollups stop
   being enough and an external historian datasource is warranted (verify at real
   cardinality).
2. **SurrealDB storage engine on edge** — embedded backend choice (durability/ops),
   given SurrealKV maturity.
3. **Rhai rule vocabulary** — confirm time-windowed conditions and hysteresis/
   debounce ("for N minutes") split cleanly: DataFusion computes the window, Rhai
   makes the decision.
4. **Live-query fan-out limits** — scale ceiling of SurrealDB live queries as the
   data-change bus under many subscribers; where Zenoh takes over.
5. **Extension scope enforcement on the bus** — how the access gate filters a
   Zenoh data subscription by the extension's grants without a per-message gate
   cost.
6. **Reusable cloud-only insight/validation surface** — whether to harvest an
   existing Apache-2.0 eval/labeling frontend + schema for the cloud "prove a rule
   before deploy" loop, or build it native.
7. **Undo collaboration model** — per-user linear undo per resource vs. shared/
   collaborative undo, given the audit log is the global truth either way.
8. **Trace retention and sampling** — what to keep, for how long, and the sampling
   rate, given rule-run and data-flow spans are high volume.
