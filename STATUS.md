# Rubix BMS Platform — Build Status

Status of the rubix BMS/EMS backend (`rubix/`, a standalone Cargo workspace,
not part of the awaken workspace). Measured against
[STACK-DEISGN.md](STACK-DEISGN.md). Reflects the code as reviewed, not intent.

**Last reviewed:** 2026-06-12 · **Workspace:** builds clean, `cargo clippy`
clean, **78 tests passing** across 6 crates.

---

## Crate map

| Crate | Role | State |
| --- | --- | --- |
| `rubix-core` | Domain model: sites/equips/points, tags, 16-level priority array, history, sparks | Complete |
| `rubix-driver` | Driver extension contract: manifest + capability scoping (types + enforcement logic) | Complete (contract only) |
| `rubix-query` | DataFusion SQL surface over SQLite; time-bucketed `his` rollups | Complete (SQLite-only) |
| `rubix-flow` | reflow engine integration: custom nodes + board JSON loader | Library built, **not executed** |
| `rubix-tools` | awaken AI tools: read_point / write_point / query (TypedTool) | Built, **no agent loop** |
| `rubix-server` | axum HTTP API, SQLite store, zenoh data plane, supervisor, tool registry | Partially wired (see below) |

File-layout discipline holds: no source file exceeds 400 lines.

---

## Done

### HTTP API + store (`rubix-server`, `rubix-core`) — running
- Sites / equips / points CRUD with Haystack tag filters; sparks create/list/ack.
- Priority-array command path: `POST /points/{id}/write` (slot), `DELETE
  /points/{id}/write/{priority}` (relinquish), `POST /points/{id}/cur` (sensor
  ingest). Every effective-value change lands in history.
- Agent writes below `RUBIX_AI_MIN_PRIORITY` are rejected (403) at the HTTP layer.
- Pooled SQLite (rusqlite/r2d2, WAL, FK cascades). Points return their zenoh keyexpr.
- 16-level BACnet priority array verified correct (lower level number wins,
  relinquish restores fallback, range-checked).
- OpenAPI 3.1 via utoipa at `/api-docs/openapi.json`.

### Zenoh data plane (`rubix-server/bus`) — running
- Opens a zenoh session in `main` (toggle `RUBIX_ZENOH=0`).
- Publishes live `cur` values on `{org}/{site}/{equip}/{point}/cur` on every
  ingest, write, and relinquish (including bus-driven writes).
- Serves `**/write` (priority-array command) and `**/his/**` (history)
  queryables against the store. 3 integration tests over real peer sessions.

### DataFusion query surface (`rubix-query`) — running
- `QueryEngine` registers canonical tables (`sites`, `equips`, `points`, `his`,
  `sparks`) as read-only DataFusion `TableProvider`s over SQLite; schema read
  live from `PRAGMA table_info` so empty tables still resolve columns.
- `POST /api/v1/query` (gated by `RUBIX_QUERY`); `POST /api/v1/his/rollup` for
  time-bucketed aggregates (avg/min/max/sum/count/first/last × minute…week),
  epoch-aligned buckets, SQL-injection guard on point ids.

### Driver contract (`rubix-driver`) — types complete
- `DriverManifest` (identity, contributed point types, capabilities, config)
  with fail-closed validation.
- `CapabilitySet` confines a session to granted keyexpr prefixes with
  publish/subscribe gating and correct path-boundary matching.

### Driver supervisor (`rubix-server/supervisor`) — built, NOT launched
- Spawn manifest-described child processes (env-injected identity/caps/config),
  liveliness-token health (`await_attach` / `await_clear` reaps orphans),
  jittered exponential backoff, shared shutdown signal, fail-closed on bad manifest.
- **Gap:** never called from `main` — no drivers are actually supervised yet.

### reflow engine (`rubix-flow`) — library built, NOT executed
- `PointAccess` port; custom nodes `read_point`, `write_point` (always through
  the priority array), `query_his`. `BoardGraph` JSON format + `load()` →
  runnable reflow `Network`. `StorePointAccess` implements the port over the store.
- reflow pinned to crates.io `0.2` (MIT/Apache) — **no fork**; custom nodes via
  the `Actor` trait.
- **Gap:** no caller ever runs a loaded `Network`. Nodes are tested via their
  behavior closures, not inside a running network. No board-deploy/run endpoint.

### awaken AI tools (`rubix-tools` + `rubix-server/tools`) — built, NO agent
- `read_point` (read-only), `write_point` (priority-array gated, refuses below
  `ai_min_priority` with the store untouched), `query` (read-only SELECT/WITH only).
  Each is a `TypedTool` over crates.io `awaken-runtime-contract 0.6`.
- `build_tools(&AppState)` constructs the store/engine-backed tool set;
  integration-tested end-to-end against a real store + the priority gate.
- **Gap:** no embedded awaken runtime/agent consumes `build_tools()` — the tools
  are reachable but nothing calls them as an LLM agent.

---

## Not started / remaining (per STACK-DEISGN.md)

### Wiring gaps (code exists, not connected)
- [ ] **Launch the supervisor** from `main` (drivers spawned at boot).
- [ ] **Embed an awaken runtime** and register `build_tools()` (an actual agent loop).
- [ ] **Board execution**: deploy/run reflow boards (load → run a `Network`);
      no endpoint or scheduler runs them.

### Engine: reflow boards
- [ ] Control boards (edge): triggered by zenoh subscriptions — schedules,
      setpoints, sequencing. Node palette from actor manifests.
- [ ] Rule boards / sparks (cloud): scheduled, query history + live values,
      **publish findings on `{org}/{site}/spark/{rule}/**`** and persist.
      Sparks are currently HTTP-CRUD only — not published on zenoh, not scheduled.
- [ ] `agent_call` and finding-emitter actors.
- [ ] Versioned board storage + zenoh deploy to stations (hot-reload).

### AI layer (awaken)
- [ ] `run_board` and `pin_widget` tools (only read_point/write_point/query exist).
- [ ] **HITL escalation**: writes above the configured priority threshold suspend
      for approval via awaken's run-suspension model (currently a hard refusal).
- [ ] **Inbound dispatch**: spark findings activate agent runs via the mailbox
      ("simultaneous heat/cool on AHU-3" arrives as a job, not a chat).
- [ ] **Outbound adapters**: MCP / A2A / AG-UI expose the building to external
      agents with the same gating. None present.
- [ ] Tenancy: org/site hierarchy mirrored into awaken `ScopeId`.

### Query / history tiering
- [ ] `points_cur` `TableProvider` backed by zenoh `get` (live values in SQL).
- [ ] `his` `TableProvider` over **Parquet partitions** via `object_store`
      (edge + cloud tiers). History is SQLite-only today — no Parquet, no tiering.
- [ ] Postgres federation (cloud relational tables: users, teams, config).
- [ ] Flight SQL surface.

### Driver runtime (beyond the contract)
- [ ] Capability enforcement **at the bus**: scoped zenoh session per driver
      limited to granted prefixes. The `CapabilitySet` logic exists in
      `rubix-driver` but is not yet applied to the server's queryables (which
      still answer global `**/write`, `**/his/**`).
- [ ] Ack/backpressure protocol for writes; bounded buffers with declared
      overflow policy (drop-oldest `cur`, reliable `write`/`his`).
- [ ] A reference driver binary.

### Platform / deployment
- [ ] **Edge/cloud profiles**: single binary, cargo-feature + runtime-config
      selected. No feature split exists yet (one build, all features compiled in).
- [ ] **Auth**: OIDC/JWT middleware, RBAC org→team→site scoping, PATs/service
      accounts. None present.
- [ ] **UI**: React flow canvas + dashboard pages, served by axum. None present.
- [ ] Postgres backend for the cloud profile (SQLite-only today).

---

## Known issues / cleanups
- `rubix-server` queryables declare global `**/write` and `**/his/**`; in a
  multi-node mesh every node answers every query (replies "not found" for points
  it doesn't own). Scope queryables to owned/granted prefixes when driver
  identity lands (ties to the bus-capability-enforcement item above).
- `rubix-query` `sql/run.rs` swallows a JSON decode error with
  `unwrap_or_default()` → empty result instead of surfacing the failure.
- No live end-to-end test of supervisor spawn/restart (needs a real driver
  binary) or of a running reflow board.

---

## Build & test

```
cd rubix
cargo test --workspace     # 78 passing
cargo clippy --workspace --all-targets
```

Env: `RUBIX_DB`, `RUBIX_ADDR`, `RUBIX_AI_MIN_PRIORITY`, `RUBIX_ZENOH` (0=off),
`RUBIX_QUERY` (0=off).
