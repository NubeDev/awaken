# Rubix BMS Platform — Build Status

Status of the rubix BMS/EMS backend (`rubix/`, a standalone Cargo workspace,
not part of the awaken workspace). Measured against
[STACK-DEISGN.md](STACK-DEISGN.md). Reflects the code as reviewed, not intent.

**Last reviewed:** 2026-06-12 · **Workspace:** builds clean, `cargo clippy`
clean, **89 tests passing** across 6 crates.

---

## Crate map

| Crate | Role | State |
| --- | --- | --- |
| `rubix-core` | Domain model: sites/equips/points, tags, 16-level priority array, history, sparks | Complete |
| `rubix-driver` | Driver extension contract: manifest + capability scoping (types + enforcement logic) | Complete (contract only) |
| `rubix-query` | DataFusion SQL surface over SQLite; time-bucketed `his` rollups | Complete (SQLite-only) |
| `rubix-flow` | reflow engine integration: custom nodes + board JSON loader + single-shot runner | Built **and executed** (via `/boards/run`) |
| `rubix-tools` | awaken AI tools: read_point / write_point / query / run_board (TypedTool) | Built, **driven by the embedded agent** |
| `rubix-server` | axum HTTP API, SQLite store, zenoh data plane, supervisor, tool registry, embedded agent | Wired (see below) |

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
  queryables against the store, **scoped to owned sites**: a node answers only
  for keys under a `{org}/{site}` it holds (read live from the store, reusing
  `Capability::covers`) and stays silent otherwise — no "not found" noise in a
  multi-node mesh. Sites provisioned after boot are covered without re-declaring.
- Publishes spark findings on `{org}/{site}/spark/{rule}/{id}` on `POST /sparks`
  so cloud subscribers (alerting, agent dispatch) observe findings live.
- Integration tests over real peer sessions (cur pub, write/his queryables,
  spark publish, unowned-site silence).

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

### Driver supervisor (`rubix-server/supervisor`) — launched at boot
- Spawn manifest-described child processes (env-injected identity/caps/config),
  liveliness-token health (`await_attach` / `await_clear` reaps orphans),
  jittered exponential backoff, shared shutdown signal, fail-closed on bad manifest.
- `main` loads manifests from `RUBIX_DRIVERS` (JSON array, default `drivers.json`)
  and calls `Supervisor::launch` on the bus session. Missing file → no drivers
  (valid for cloud nodes); malformed file fails closed. Ctrl-C graceful shutdown
  stops drivers so liveliness tokens clear before exit.
- **Remaining:** no live end-to-end spawn/restart test (needs a reference driver
  binary).

### reflow engine (`rubix-flow`) — executed via `/boards/run`
- `PointAccess` port; custom nodes `read_point`, `write_point` (always through
  the priority array), `query_his`. `BoardGraph` JSON format + `load()` →
  runnable reflow `Network`. `StorePointAccess` implements the port over the store.
- `BoardGraph::run()` does a single-shot evaluation: start the network, tick
  source nodes, settle, collect every node's outport output. `POST
  /api/v1/boards/run` runs an inline board over the store and returns the outputs;
  integration-tested (read → priority-array write reaches the store).
- reflow pinned to crates.io `0.2` (MIT/Apache) — **no fork**; custom nodes via
  the `Actor` trait.
- **Remaining:** no scheduler or zenoh-subscription trigger runs boards on a
  cadence yet; `/boards/run` is caller-driven. No versioned board storage.

### awaken AI tools + embedded agent (`rubix-tools` + `rubix-server`) — agent loop live
- `read_point` (read-only), `write_point` (priority-array gated, refuses below
  `ai_min_priority` with the store untouched), `query` (read-only SELECT/WITH only),
  `run_board` (evaluates a board over the store; writes go through the priority
  array). Each is a `TypedTool` over crates.io `awaken-runtime-contract 0.6`.
- `build_tools(&AppState)` constructs the store/engine-backed tool set, handed to
  an embedded `awaken_runtime::AgentRuntime` (crates.io `awaken-runtime 0.6`).
  `POST /api/v1/agent/chat` runs a tool-calling agent turn. Enabled by `RUBIX_AI=1`;
  model/provider/rounds env-selected (`RUBIX_AI_PROVIDER`/`MODEL_ID`/`MODEL`/
  `MAX_ROUNDS`). The genai provider reads its key at run time, so the node boots
  without one. Offline integration test drives the loop via a scripted executor.
- **Remaining:** HITL escalation, inbound spark-finding dispatch via the mailbox,
  and `pin_widget` are not yet wired (see below).

---

## Not started / remaining (per STACK-DEISGN.md)

### Wiring gaps — CLOSED this pass
- [x] **Launch the supervisor** from `main` (drivers spawned at boot from `RUBIX_DRIVERS`).
- [x] **Embed an awaken runtime** and register `build_tools()` — `POST /agent/chat`.
- [x] **Board execution**: `BoardGraph::run()` + `POST /boards/run`.
- [x] **Spark zenoh publishing** on `{org}/{site}/spark/{rule}/{id}` at create.
- [x] **Bus capability scoping**: queryables answer only for owned sites.
- [x] **`run_board` agent tool** over the store.

### Engine: reflow boards
- [ ] Control boards (edge): **scheduled or zenoh-subscription triggered** —
      `/boards/run` runs a board once on demand, but nothing fires boards on a
      cadence or off a `cur` subscription yet. Node palette from actor manifests.
- [ ] Rule boards / sparks (cloud): **scheduled** evaluation that queries history
      + live values and emits sparks. Sparks publish on zenoh now, but no
      scheduler runs rule boards to produce them.
- [ ] `agent_call` and finding-emitter actors.
- [ ] Versioned board storage + zenoh deploy to stations (hot-reload).

### AI layer (awaken)
- [ ] `pin_widget` tool (read_point/write_point/query/run_board exist).
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
- [~] Capability enforcement **at the bus**: the server's queryables are now
      scoped to owned sites (a node only answers for keys it holds). Still
      remaining: a *scoped zenoh session per driver* limited to that driver's
      granted prefixes (publish/subscribe gating on the driver's own session).
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
- Queryables still *declare* global `**/write` / `**/his/**`; ownership is now
  enforced by filtering replies (the handler stays silent for non-owned keys)
  rather than by narrowing the declaration. Functionally a single responder per
  key, but a foreign query still wakes every node's handler for the ownership
  check. Per-prefix declaration would avoid that wakeup.
- `rubix-query` `sql/run.rs` swallows a JSON decode error with
  `unwrap_or_default()` → empty result instead of surfacing the failure.
- No live end-to-end test of supervisor spawn/restart (needs a real driver
  binary). Board execution is now tested (single-shot run over the store), but
  not a scheduled/triggered board.
- The embedded agent is exercised offline via a scripted LLM executor; no live
  provider test (needs an API key).

---

## Build & test

```
cd rubix
cargo test --workspace     # 89 passing
cargo clippy --workspace --all-targets
```

Env:
- `RUBIX_DB`, `RUBIX_ADDR`, `RUBIX_AI_MIN_PRIORITY`
- `RUBIX_ZENOH` (0=off), `RUBIX_QUERY` (0=off)
- `RUBIX_DRIVERS` (driver-manifest JSON path; default `drivers.json`)
- `RUBIX_AI` (1=embed agent), `RUBIX_AI_PROVIDER`, `RUBIX_AI_MODEL_ID`,
  `RUBIX_AI_MODEL`, `RUBIX_AI_MAX_ROUNDS`
