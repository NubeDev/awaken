# Stack Design — AI Building Automation Platform

A BMS/EMS platform merging the roles of Niagara (supervisory control, drivers,
wiresheet) and SkySpark (historian, analytics rules, findings) with a native AI
agent layer. Single Rust binary, runs from Raspberry Pi-class edge devices to
cloud, offline-capable per site.

## Concept mapping

| Legacy concept            | This platform                                          |
| ------------------------- | ------------------------------------------------------ |
| Niagara Station           | Edge node: zenoh router + driver PIDs + reflow boards  |
| FOX network               | Zenoh mesh (edge↔edge↔cloud), keyexprs as addressing   |
| Wiresheet                 | reflow graphs (control logic), own React canvas UI     |
| Drivers (BACnet/Modbus)   | Supervised child processes speaking zenoh              |
| SkySpark Folio + Axon     | Parquet/SQLite history + DataFusion SQL                |
| Sparks (rule findings)    | Scheduled reflow boards publishing findings on zenoh   |
| Haystack tags             | Tag registry mapped onto zenoh key expressions         |
| (no equivalent)           | awaken agents: diagnose, propose, gated point writes   |

## Component choices

| Layer                  | Choice                                        | License    |
| ---------------------- | --------------------------------------------- | ---------- |
| Flow/rules engine      | reflow (`offbit-ai/reflow`) actor core        | MIT        |
| AI agent runtime       | awaken (this repo), embedded library mode     | MIT/Apache |
| Transport + ext bus    | zenoh                                         | EPL/Apache |
| Query/analytics        | DataFusion + datafusion-table-providers       | Apache-2.0 |
| Relational store       | Postgres (cloud), SQLite (edge)               | —          |
| History store          | Parquet via object_store (edge + cloud tiers) | Apache-2.0 |
| HTTP layer             | axum                                          | MIT        |
| UI                     | Own React app: flow canvas + dashboard pages  | own        |

**flow-like is a design reference only.** It is BSL-licensed (TM9657 GmbH,
non-compete grant). No code is copied, ported, or depended on. Reading its
architecture is fine; translating its source is not — translated code remains
BSL. The local clone must never be committed to this repository.

**reflow** was chosen over flow-like (license), Windmill (heavy, Pi-hostile,
no dashboard path), and the rest of the review field (z8run one-shot model,
EdgeLinkd alpha, Arroyo/RisingWave server-class). Engine facts: ~13k LOC core,
actor model on bounded flume channels with reliable/latest delivery semantics,
`#[actor]` macro for custom actors, StreamHandle for binary payloads, MIT and
forkable. Known gaps owned by us: no visual editor (Zeal is closed-source),
no auth, no dashboards. Single-author upstream; mitigation is a fork we can
maintain (13k LOC core).

## Topology

```
                                  ┌──────────────────────────────────────────────────┐
                                  │            CLOUD / SUPERVISOR  (one binary)      │
                                  │                                                  │
   Browser / Mobile               │  ┌────────────────────────────────────────────┐  │
  ┌────────────────┐    HTTPS/SSE │  │ axum: API · auth (OIDC/RBAC) · dashboards  │  │
  │ React UI       │◄────────────►│  └───────┬───────────────┬──────────────┬─────┘  │
  │ · flow editor  │              │          │               │              │        │
  │   (canvas)     │              │  ┌───────▼──────┐ ┌──────▼─────┐ ┌──────▼─────┐  │
  │ · dash pages   │              │  │ reflow       │ │ awaken     │ │ datafusion │  │
  └────────────────┘              │  │ engine (MIT) │ │ AI agents  │ │ + table-   │  │
                                  │  │ rules/sparks │◄►│ tools:    │◄►│ providers │  │
   Claude / external agents       │  │ insights     │ │ run_board  │ │ SQL over   │  │
  ┌────────────────┐   MCP/A2A    │  └───────┬──────┘ │ query/write│ │ pg+history │  │
  │ MCP clients    │◄────────────►│          │        └──────┬─────┘ └──────┬─────┘  │
  └────────────────┘              │          │               │              │        │
                                  │  ┌───────▼───────────────▼──────────────▼─────┐  │
                                  │  │              zenoh session                 │  │
                                  │  └───────────────────┬────────────────────────┘  │
                                  │   Postgres ──────────┤  Parquet/object store     │
                                  └──────────────────────┼───────────────────────────┘
                                                         │
                                            zenoh mesh   │   (TLS, works offline,
                                          edge ↔ cloud   │    store-and-forward)
                                                         │
            ┌────────────────────────────────────────────┼────────────────────────────┐
            │                EDGE / Pi "station"  (same binary, slim features)        │
            │                                            │                            │
            │  ┌─────────────────────────────────────────▼─────────────────────────┐  │
            │  │            zenoh router (shared-memory transport on-box)          │  │
            │  └───────┬──────────────┬───────────────┬──────────────────┬─────────┘  │
            │          │              │               │                  │            │
            │  ┌───────▼──────┐  ┌────▼─────────┐ ┌───▼──────────┐ ┌─────▼─────────┐  │
            │  │ reflow       │  │ local store  │ │ supervisor   │ │ (optional)    │  │
            │  │ engine       │  │ SQLite +     │ │ spawn/health │ │ awaken mini   │  │
            │  │ control      │  │ Parquet      │ │ restart/     │ │ agent         │  │
            │  │ boards, PID  │  │ history      │ │ backoff      │ │ (remote LLM)  │  │
            │  └──────────────┘  └──────────────┘ └───┬──────────┘ └───────────────┘  │
            └─────────────────────────────────────────┼───────────────────────────────┘
                                                      │ separate PIDs (crash-isolated)
                                 ┌────────────┬───────┴──────┬─────────────┐
                                 │            │              │             │
                           ┌─────▼─────┐ ┌────▼──────┐ ┌─────▼─────┐ ┌─────▼─────┐
                           │ BACnet    │ │ Modbus    │ │ MQTT/KNX  │ │ custom    │
                           │ driver    │ │ driver    │ │ driver    │ │ driver    │
                           └─────┬─────┘ └────┬──────┘ └─────┬─────┘ └─────┬─────┘
                                 │            │              │             │
                            field devices: AHUs · VAVs · meters · sensors · plant
```

## Single binary, two profiles

One executable; cargo features and runtime config select the profile.

- **Edge (Pi):** zenoh peer/router, driver supervisor, reflow control boards,
  SQLite + local Parquet history with ring-buffer retention and cloud tiering.
  No Postgres, no full analytics, optional thin awaken agent (remote LLM).
- **Cloud / supervisor:** zenoh router, Postgres (auth, teams, config, awaken
  stores), full DataFusion context over object storage, scheduled rule boards,
  awaken server with all protocol adapters, React UI hosting.
- **All-in-one:** both profiles on one box for small sites.

The only separate processes anywhere are protocol drivers (hard requirement:
crash isolation from native protocol stacks).

## Zenoh: addressing and data plane

Key expression scheme is the point identity (Haystack-flavoured):

```
{org}/{site}/{equip-path}/{point}/cur      live value        pub/sub
{org}/{site}/{equip-path}/{point}/write    command           queryable, priority-array semantics
{org}/{site}/{equip-path}/{point}/his/**   history           queryable, served by local store
{org}/{site}/spark/{rule}/**               rule findings     pub/sub + persisted
```

- Tags (Project Haystack 4 defs: `ahu`, `discharge`, `temp`, `sensor`, …) live
  in a registry keyed by the same path; tag queries resolve to keyexpr sets.
- Point writes keep BACnet's 16-level priority array. AI writes enter at a
  configured low priority; operator overrides always win.
- Edge answers its own `his` queries; cloud is just another peer with a bigger
  store. Offline operation falls out of the topology, not special-case code.
- On-box, extensions and the host share the zenoh shared-memory transport.

## Driver extension contract

Drivers are supervised child processes. The contract (separate spec doc owns
the detail) covers:

- **Manifest:** identity, protocol, contributed point types/pin schemas,
  required capabilities (publish/subscribe keyexpr prefixes), config schema.
- **Supervisor:** spawn, health (liveliness token on zenoh), restart with
  exponential backoff + jitter, stale-process reaping at boot.
- **Capability enforcement at the bus:** each driver gets a scoped zenoh
  session limited to its granted keyexpr prefixes.
- **Ack/backpressure:** explicit ack for writes; bounded buffers with declared
  overflow policy (drop-oldest for `cur`, reliable for `write`/`his`).

## Engine: reflow

- Custom actors via `#[actor]`: zenoh subscribe/publish, point read,
  point write (always through the priority array, never raw), history query,
  DataFusion SQL, `agent_call` (invoke an awaken agent), finding emitter.
- **Control boards** (edge): triggered by zenoh subscriptions; supervisory
  control only — schedules, setpoints, resets, sequencing. Fast loops stay in
  field controllers; this is not a PLC.
- **Rule boards / sparks** (cloud): scheduled, query history + live values,
  publish findings to `spark/**` and persist them to a `sparks` table.
  Same canvas authors both — one editor for control and analytics.
- Graphs stored as JSON in Postgres/SQLite, versioned; deployed to stations
  over zenoh.

## Query layer: DataFusion

DataFusion is the single SQL surface; dashboards never execute flow graphs to
fetch data.

- Canonical schema: `sites`, `equips`, `points` (tags as columns/maps), `his`,
  `sparks`, plus relational tables (users, teams, config) federated via
  datafusion-table-providers (Postgres).
- Custom `TableProvider`s: `points_cur` backed by zenoh `get` (live values),
  `his` backed by Parquet partitions, served locally or via zenoh queryables
  from edge stores.
- Exposed over HTTP (and later Flight SQL) for dashboards; the same context is
  available to reflow actors and awaken tools — one engine, three consumers.

## AI layer: awaken

Embedded runtime-library mode in the host process; awaken's provider layer is
the single LLM gateway for the platform.

- **Tools:** `query` (DataFusion SQL), `read_point`, `write_point` (gated:
  tool permission + priority array + HITL escalation above a configured
  priority — awaken's run suspension model handles the approval flow),
  `run_board`, `pin_widget`.
- **Inbound:** spark findings dispatch run activations via the mailbox —
  "simultaneous heat/cool on AHU-3" arrives as a job, not a chat.
- **Outbound:** awaken's MCP/A2A/AG-UI adapters expose the building to
  external agents and clients with the same gating.
- Tenancy: org/site hierarchy mirrors into awaken `ScopeId`.

## UI (owned)

React application served by axum:

- **Flow canvas:** React Flow-based editor for reflow graphs (z8run's MIT
  editor is a vendorable starting point). Node palette generated from actor
  manifests, including driver-contributed nodes.
- **Dashboard pages:** server-stored JSON page/widget definitions rendered by
  a widget registry (charts, tables, maps, forms); widgets bind to DataFusion
  queries, live widgets via SSE off zenoh subscriptions. JSON-definition
  approach keeps a future mobile renderer possible.
- **Auth:** OIDC (any issuer; Keycloak/Zitadel for self-hosted), JWT
  middleware on axum, RBAC with org → team → site/app scoping, PATs and
  service accounts for machine access.

## Licensing rules (hard)

1. This repo and the platform stay MIT/Apache end to end.
2. flow-like (BSL) is never a dependency and never a copy source; clone stays
   untracked and gitignored.
3. reflow is consumed as a pinned git/crates dependency; if upstream stalls,
   fork under MIT and maintain.

## Open questions / spikes

- reflow on Pi at telemetry rate: actor graph with ~1k points, sustained
  pub rate, memory ceiling.
- reflow graph hot-reload / versioned deploy semantics (engine supports
  programmatic load; the deploy story is ours).
- Zenoh built-in storage vs owned historian: decide the replication mechanism
  for edge→cloud `his` tiering.
- Editor: how much of z8run's canvas survives contact with reflow's graph
  model (ports, subgraphs, IIPs).
- Live widget fan-out: SSE per widget vs multiplexed per page.
- Storage per tier final call: SQLite+Parquet edge confirmed; cloud Parquet
  on object_store vs Timescale for `his`.
