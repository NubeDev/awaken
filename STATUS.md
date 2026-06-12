# Rubix BMS Platform — Build Status

Status of the rubix BMS/EMS backend (`rubix/`, a standalone Cargo workspace,
not part of the awaken workspace). Measured against
[STACK-DEISGN.md](STACK-DEISGN.md). Reflects the code as reviewed, not intent.

**Last reviewed:** 2026-06-12 · **Workspace:** builds clean, `cargo clippy`
clean, **97 tests passing** across 6 crates.

---

## Crate map

| Crate | Role | State |
| --- | --- | --- |
| `rubix-core` | Domain model: sites/equips/points, tags, 16-level priority array, history, sparks | Complete |
| `rubix-driver` | Driver extension contract: manifest + capability scoping (types + enforcement logic) | Complete (contract only) |
| `rubix-driver-sim` | Reference driver binary: capability-scoped zenoh `cur` simulator | Built, **spawned by the supervisor in a live test** |
| `rubix-query` | DataFusion SQL surface over SQLite; time-bucketed `his` rollups | Complete (SQLite-only) |
| `rubix-flow` | reflow engine integration: custom nodes + board JSON loader + single-shot runner | Built **and executed** (inline `/boards/run`, stored boards, scheduler) |
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
- **Live spawn covered**: the `rubix-driver-sim` reference binary (below) is the
  spawn target for an integration test — the real `Supervisor` spawns it, it
  attaches (liveliness token), publishes `cur` a second peer receives, and the
  token clears on shutdown. The previously-untested end-to-end spawn path.

### Reference driver (`rubix-driver-sim`) — runs under the supervisor
- A real, capability-scoped simulator driver (not a test stub): reads the
  driver contract from env (`RUBIX_DRIVER_NAME`/`_CAPS`/`_CONFIG`, now defined
  canonically in `rubix-driver` and re-exported by the supervisor), opens a peer
  zenoh session, declares its liveliness token, and publishes a simulated `cur`
  oscillation on a granted point keyexpr on a cadence. **Self-authorizes** each
  publish against its `CapabilitySet` (fails closed at startup if the configured
  point is outside its grant), and clears its token on SIGINT so the supervisor
  reaps it. Config: `{ point, period_secs, baseline, amplitude }`.

### reflow engine (`rubix-flow`) — executed via `/boards/run`
- `PointAccess` port; custom nodes `read_point`, `write_point` (always through
  the priority array), `query_his`, `emit_spark` (records a rule finding via
  `PointAccess::emit_spark`), `agent_call` (raises an embedded-agent run via
  `PointAccess::request_agent` detached, or `request_agent_blocking` when
  `await: true` to surface the run outcome on `output`). `BoardGraph` JSON format +
  `load()` → runnable reflow `Network`. `StorePointAccess` implements the port
  over the store.
- `BoardGraph::run()` does a single-shot evaluation: start the network, tick
  source nodes, settle, collect every node's outport output. `POST
  /api/v1/boards/run` runs an inline board over the store and returns the outputs;
  integration-tested (read → priority-array write reaches the store).
- reflow pinned to crates.io `0.2` (MIT/Apache) — **no fork**; custom nodes via
  the `Actor` trait.

### Board scheduler + versioned storage (`rubix-server/scheduler` + `store/boards`) — running
- **Versioned board storage**: `boards` table keyed by `(slug, version)` UNIQUE;
  `create_board` inserts a fresh version, `get_board`/`latest_boards` return the
  highest version per slug. CRUD over HTTP: `POST /api/v1/boards` (create/
  republish), `GET /api/v1/boards`, `GET|DELETE /api/v1/boards/{slug}`,
  `POST /api/v1/boards/{slug}/run` (run a stored board on demand).
- **Scheduler**: one detached task per scheduled board, watch-channel shutdown
  (mirrors the driver supervisor). `Trigger::Interval { seconds }` fires on a
  cadence; `Trigger::Subscription { key }` fires on each `cur` sample matching a
  zenoh keyexpr; `Trigger::Manual` is run-only. Each loop re-reads its board from
  the store when its trigger fires, so republishing/disabling takes effect on the
  next tick without restarting the scheduler. Disabled boards are stored but
  never fired.
- `main` launches the scheduler at boot from the store's scheduled boards
  (toggle `RUBIX_SCHEDULER=0`); subscription boards need the bus session and are
  skipped with a warning when `RUBIX_ZENOH=0`. Graceful shutdown drains the
  loops before drivers stop. Integration-tested: CRUD + versioning, run-by-slug,
  and a live 1s-interval board commanding a point over the store.
- **Rule boards emit sparks**: the `emit_spark` node records a finding via
  `PointAccess::emit_spark` (config `site`/`rule`/`severity`/`message`, or the
  message from a `value` inport). The store-backed impl resolves the `{org}/
  {site}` prefix to a site id, persists the spark, and — when a bus is present —
  **publishes it on `{org}/{site}/spark/{rule}/{id}`** (detached, best-effort),
  the same keyexpr scheme as HTTP `POST /sparks`. The bus is threaded through
  the scheduler (`Scheduler::launch` takes `Option<ZenohBus>`) and both
  `/boards/run` paths. Integration-tested: board run → spark persisted + a
  second peer session receives the publish.
- **Remaining:** the running scheduler is not hot-reconfigured — a board added
  after boot is picked up on the next restart (no live add/remove of loops yet).
  No zenoh deploy of boards to stations (hot-reload).

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
- **Inbound spark dispatch** (`dispatch` module): a `Dispatcher` subscribes to
  `**/spark/**` on the bus and activates the agent per finding — a *job*, not a
  chat. Each spark becomes a `RunActivation` on a thread keyed by the spark id,
  with an investigate-then-act-within-gating prompt; the run's tool calls hit
  the same gated point/history tools. Launched from `main` when both bus and
  agent are present (`RUBIX_AI_DISPATCH`), graceful shutdown stops it before the
  scheduler/drivers. Integration-tested: a published spark drives a scripted run
  that commands a point. So the rule-board → spark → bus → agent-run loop is now
  closed end to end.
- **Remaining:** see the AI-layer list below for HITL escalation and `pin_widget`
  status. Dispatch runs are fire-and-log — no run registry/operator surface for
  in-flight dispatched runs yet, and a suspended (awaiting-approval) dispatched
  run is logged with its id but has no resume UI.

---

## Not started / remaining (per STACK-DEISGN.md)

### Wiring gaps — CLOSED this pass
- [x] **Launch the supervisor** from `main` (drivers spawned at boot from `RUBIX_DRIVERS`).
- [x] **Embed an awaken runtime** and register `build_tools()` — `POST /agent/chat`.
- [x] **Board execution**: `BoardGraph::run()` + `POST /boards/run`.
- [x] **Spark zenoh publishing** on `{org}/{site}/spark/{rule}/{id}` at create.
- [x] **Bus capability scoping**: queryables answer only for owned sites.
- [x] **`run_board` agent tool** over the store.
- [x] **Board scheduler**: interval + `cur`-subscription triggers fire stored
      boards (`RUBIX_SCHEDULER`), launched from `main`.
- [x] **Versioned board storage** + CRUD HTTP API + run-by-slug.

### Engine: reflow boards
- [x] Control boards (edge): **scheduled or zenoh-subscription triggered** — the
      scheduler fires stored boards on an `Interval` cadence or off a `cur`
      `Subscription`. Node palette from actor manifests still TODO.
- [x] Rule boards / sparks (cloud): scheduled evaluation runs, the `emit_spark`
      node records a finding, and board sparks publish on the bus (same scheme
      as HTTP `/sparks`).
- [x] `agent_call` actor (board step that invokes the embedded agent). Two modes
      via the `await` config flag: detached (control-board default — the run
      proceeds out-of-band, the node acknowledges) and awaited (the single-shot
      run blocks on the run and surfaces the agent's decision on `output` so a
      downstream node branches on it; `request_agent_blocking` bridges the sync
      port to the async runtime). Fails closed without an agent runtime
      (recursion guard for the agent's own `run_board` tool).
- [x] Versioned board storage (`boards` table, `(slug, version)`). **Remaining:**
      zenoh deploy to stations (hot-reload), and live scheduler reconfiguration
      (added boards picked up on restart, not hot).

### AI layer (awaken)
- [x] `pin_widget` tool — agent pins a dashboard tile (`point_value`/
      `point_history`/`board_output`) on a site; store-backed (`widgets` table),
      also exposed as `POST`/`GET /api/v1/widgets`.
- [~] **HITL escalation**: `write_point` now bands by slot — at/below the agent
      ceiling commits, the **escalation band** (`RUBIX_AI_ESCALATION_FLOOR`..ceiling)
      returns a *suspended* `ToolResult` with a `SuspendTicket` (run terminates
      `Suspended`, the store untouched), and slots below the floor are hard-refused.
      `POST /agent/chat` surfaces this as `status: awaiting_approval` + the `run_id`.
      **Remaining:** the resume endpoint (operator approve/cancel) needs the
      persistent-backend run store — lands with the mailbox/dispatch layer.
- [x] **Inbound dispatch**: the `dispatch` module subscribes to `**/spark/**`
      and activates an agent run per finding — a job, not a chat ("simultaneous
      heat/cool on AHU-3" arrives as a `RunActivation` on a spark-keyed thread).
      Launched from `main` when bus+agent present (`RUBIX_AI_DISPATCH`). See the
      embedded-agent section above. **Remaining:** persistent run registry /
      operator surface for in-flight and suspended dispatched runs.
- [ ] **Outbound adapters**: MCP / A2A / AG-UI expose the building to external
      agents with the same gating. None present.
- [ ] Tenancy: org/site hierarchy mirrored into awaken `ScopeId`.

### Query / history tiering
- [~] `points_cur` SQL surface: a `points_cur` **view** (registered on the
      `/query` context) flattens each point's effective `cur_value`/`cur_ts` and
      joins site/equip to expose the resolved keyexpr —
      `SELECT keyexpr, cur_value FROM points_cur`. Local-store backed (cur values
      land in `points` on every write). **Remaining:** the cross-mesh variant
      backed by zenoh `get` (pull live values from peer nodes that own the keys).
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
- [x] A reference driver binary (`rubix-driver-sim`): capability-scoped zenoh
      publisher, spawned and health-checked by the supervisor in a live test.

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
- `RUBIX_DB`, `RUBIX_ADDR`, `RUBIX_AI_MIN_PRIORITY`, `RUBIX_AI_ESCALATION_FLOOR`
- `RUBIX_ZENOH` (0=off), `RUBIX_QUERY` (0=off)
- `RUBIX_DRIVERS` (driver-manifest JSON path; default `drivers.json`)
- `RUBIX_AI` (1=embed agent), `RUBIX_AI_PROVIDER`, `RUBIX_AI_MODEL_ID`,
  `RUBIX_AI_MODEL`, `RUBIX_AI_MAX_ROUNDS`
