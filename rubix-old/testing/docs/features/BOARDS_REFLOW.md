# Feature — Reflow Boards (flow runtime)

> Verified: **updated** on `rubix-gaps` (`bb775b1b`, 2026-06-14) after the
> flow-runtime redesign — persistent scan engine, async/quality-aware seam, SSE
> live bus, node-state system, and the save-refire fix. Source:
> `rubix-flow/src/{board,node,port.rs,state.rs}`,
> `rubix-server/src/{flow,scheduler,api/boards}`, `rubix-server/src/store/node_state.rs`.

Covers: the node palette, board JSON shape, the **execution model** (one-shot
runs vs the persistent scan engine), the flow↔server seam (`PointAccess`, async +
push + quality), node state across saves, every board **API** (incl. the SSE live
stream), and the scheduler (interval + `cur`-subscription triggers).

Prereq: stack up. For `agent_call` boards, `RUBIX_AI=1`. Scheduler on by default
(`RUBIX_SCHEDULER=1`). `$BASE`, `post()` from the cheatsheet.

---

## 1. How it works (read this once)

A **board** is a [reflow](https://crates.io/crates/reflow_network) actor graph:
one actor per node, wired by the graph's connections. `rubix-flow` owns the engine
wrapper and the node implementations; it carries **no** axum/sqlite/zenoh — nodes
reach the host only through the **`PointAccess`** seam (`rubix-flow/src/port.rs`),
which `rubix-server` implements over the store/bus/agent.

There are **two execution models**, and which one runs depends on how the board is
invoked:

| Path | Model | Used by |
|------|-------|---------|
| `BoardGraph::run` (`board/run.rs`) | **one-shot**: build a fresh `Network`, `start`, tick sources, drain until settled (50 ms quiet) or the 120 s budget, then `shutdown`/drop | inline `POST /boards/run`, `POST /boards/{slug}/run`, and **subscription** triggers (one run per `cur` sample) |
| `BoardEngine` (`board/engine.rs`) | **persistent scan**: one started `Network` kept alive for the board's lifetime; each scan re-ticks the sources and folds the new values into a retained snapshot — **no rebuild per tick** | **interval** scheduler loops |

The persistent engine is the big change from the old model: an "every 1 s" board
no longer constructs and demolishes its whole actor network every second. The
actor tasks (and their state) survive between scans, so the old `[INPORT CLOSED]`
churn and the `ws://localhost:8080` tracing-client reconnect storm are gone
(tracing is disabled at `load`).

**Observing link values.** reflow consumes a *connected* node's outport with an
internal fan-out forwarder, so a node whose output feeds a downstream node is
**not** drained directly. The engine therefore reads interior link values from the
network's `MessageSent` event stream and terminal (no-outbound) node values from
`read_actor_output`, giving a complete per-`(node, port)` snapshot. (The one-shot
`run()` only reliably surfaces *un-consumed* outports — see the gotcha.)

### The seam (`PointAccess`)

Every node call to the host goes through `PointAccess`. As of the redesign it is:

- **async** — `read_point` / `write_point` / `query_his` / `query_datasource` /
  `request_agent` / `request_agent_awaited` are `async fn`. The host runs SQLite on
  the blocking pool and `.await`s datasource/agent I/O (with timeouts: agent 120 s,
  datasource 60 s) instead of parking a worker.
- **typed** — errors are a `FlowAccessError` (`Unsupported` / `Denied` / `Store`),
  not opaque `anyhow`.
- **push-capable** — `watch(prefix) -> Stream<WatchSample>` is the event-driven
  counterpart to `read_point`. The scheduler's `Subscription` trigger now consumes
  `watch`, so there is **one** subscription substrate (no double-subscribe).
- **fail-closed** — `emit_spark`, `request_agent*`, `query_datasource`, `rule_store`,
  and `node_state` default to "unsupported"/`None`, so an access wired without a
  backend (the agent's own `run_board` tool) makes that capability fail closed.

### Link quality

Every value the engine/`run()` captures carries a **quality** flag derived at
capture: a value on a node's `error` port is `fault`, a JSON `null` is `null`,
else `ok`. It rides on `NodeOutput.quality` and the server's `PortOutput.quality`,
so REST snapshots and the SSE stream are self-describing (the flow editor colours a
fault port red). See `board/run.rs::Quality`.

### Node state across a save (`StatePolicy`)

A stateful node (the `trigger`'s clock, a counter…) declares **how its state
survives a republish/save and a restart** — there is no implicit default
(`rubix-flow/src/state.rs`):

| `StatePolicy` | Survives save (republish)? | Survives restart? | Backing |
|---------------|----------------------------|-------------------|---------|
| `Ephemeral` | no (resets on engine rebuild) | no | per-engine in-memory map |
| `Session` | **yes** | no | scheduler-owned, board-scoped in-memory map |
| `Durable` | **yes** | **yes** | `node_state` store table (sqlite + postgres) |

The `trigger` declares `Session`: its clock/count survive a save (it does **not**
re-fire its boot) but reset on a server restart. State is keyed by the board's
**stable** identity `(org, site_id, slug)`, not the per-version row id — that and
the scheduler keying below are the two halves of the save-refire fix.

---

## 2. The node palette (8 built-ins)

`rubix-flow/src/board/registry.rs` (`COMPONENTS`); ports/config in `src/node/`.
`GET /api/v1/boards/components` returns the live schema (ports + config fields).

| component | inports | outports | config | does |
|-----------|---------|----------|--------|------|
| `read_point` | `trigger` | `output`, `error` | `point` (keyexpr) | reads current value |
| `write_point` | `value` | `output`, `error` | `point`, `priority` (1..16, default 16) | commands through the priority array (coalesces an unchanged re-command) |
| `query_his` | `trigger` | `output`, `error` | `point`, `limit` (default 100) | recent history as JSON array |
| `datasource` | `trigger` | `output`, `error` | `datasource`, `sql`\|`named`, `params` | strict read against an external SQL datasource — see [DATASOURCE.md](DATASOURCE.md) |
| `rule` | `input` | `finding`, `clear`, `error` | `script`\|`rule`, `params`, `max_rows` | sandboxed Rhai rule over a frame — see [RULES_ENGINE.md](RULES_ENGINE.md) |
| `emit_spark` | `value`, `finding` | `output`, `error` | `site`, `rule`, `severity` (default warning), `message`, `point` (implicated keyexpr) | records a finding (+ bus publish) |
| `agent_call` | `value` | `output`, `error` | `prompt`, `thread`, `await` (default false) | raises an embedded-agent run |
| `trigger` | `trigger` | `boot`, `count`, `output`, `error` | `every`, `unit` (sec\|min\|hours) | self-paced timing source |

The `trigger` node is a **source** (its inport has no inbound edge — the scheduler/
engine ticks it) but owns its own cadence: it fires only when `every × unit` has
elapsed since its last fire, regardless of tick rate. On a fire it emits `boot`
(`true` only on the first fire while its `Session` clock is empty — i.e. after a
restart, **not** after a save), `count` (total fires), and `output` (a level that
toggles each fire). Between fires it emits nothing.

A board graph is
`{ "nodes": [{id, component, config}], "connections": [{from_node, from_port, to_node, to_port}] }`.

---

## 3. Board API reference

`rubix-server/src/api/boards/`. All board endpoints are tenant-scoped by query:
`?org=<org>` is required, `?site_id=<uuid>` optional (omit = org-level board). No
bearer token on the edge profile unless OIDC is configured.

| Method & path | Body / query | Returns |
|---------------|--------------|---------|
| `POST /api/v1/boards/run` | `{"board": <graph>}` | `{"outputs":[{node, port, value, quality}]}` — runs an **inline** graph once (unsaved edits, the live canvas) |
| `POST /api/v1/boards` | `{slug, display_name, trigger, board:<graph>, enabled?, site_id?}` | `201` `BoardView` (`version`, `graph`, …). Republishing the same slug inserts a **new version** |
| `GET /api/v1/boards?org=` | — | latest version per slug in scope |
| `GET /api/v1/boards/{slug}?org=` | — | latest version of one board |
| `PATCH /api/v1/boards/{slug}?org=` | `{display_name?, enabled?}` | edits metadata on the latest version; (un)registers its loop to match. **Republishing the graph/trigger is a new POST, not a PATCH.** |
| `DELETE /api/v1/boards/{slug}?org=` | — | `204`; unregisters the loop, clears cached outputs |
| `POST /api/v1/boards/{slug}/run?org=` | — | runs the **latest stored** version once; `{"outputs":[…]}` and records to the live cache |
| `GET /api/v1/boards/{slug}/outputs?org=` | — | latest per-`(node,port)` snapshot `[{node, port, value, quality, at}]` from the in-memory cache |
| `GET /api/v1/boards/{slug}/outputs/stream?org=` | — | **SSE**: emits the current snapshot on connect, then a fresh snapshot whenever the board runs. Same tenant authz as `/outputs` |
| `GET /api/v1/boards/components` | — | the node palette: each component's ports + config schema |
| `GET /api/v1/boards/options/{source}` | — | enum options for a config field (e.g. point/datasource pickers) |

`trigger` is internally tagged: `{"kind":"manual"}`,
`{"kind":"interval","seconds":N}` (N ≥ 1), or `{"kind":"subscription","key":"<keyexpr>"}`.

> **Wire-shape gotchas (still true):** `POST /boards/run` and `/boards/{slug}/run`
> take `{"board": <graph>}`, **not** the bare graph. `POST /boards` requires
> `display_name` and `trigger`; the graph field is `board` on the way in and `graph`
> in the response. A writable point is `kind:"sp"` (or `"cmd"`) — `"setpoint"` is not
> a valid `PointKind`.

---

## 4. Runbook

### 1. Inline run — read a point

```bash
post /api/v1/boards/run '{"board":{"nodes":[{"id":"r","component":"read_point","config":{"point":"nube/hq/ahu-3/temp"}}],"connections":[]}}' | jq
```

✅ Returns `{"outputs":[{"node":"r","port":"output","value":<cur>,"quality":"ok"}]}` —
node `r`'s current value, quality `ok`. (Node `r` has no inbound connection ⇒ it's a
**source** ⇒ it ticks.)

### 2. Inline run — read → write through the priority array

Provision `nube/hq/ahu-3/sp` as `kind:"sp"` first.

```bash
post /api/v1/boards/run '{"board":{
  "nodes":[
    {"id":"r","component":"read_point","config":{"point":"nube/hq/ahu-3/temp"}},
    {"id":"w","component":"write_point","config":{"point":"nube/hq/ahu-3/sp","priority":13}}
  ],
  "connections":[{"from_node":"r","from_port":"output","to_node":"w","to_port":"value"}]
}}' | jq
```

✅ Outputs carry node `w` on `output` with the written value; `GET /api/v1/points/$SP`
shows `priority_array.slots[12]` (slot-13) == that value and `cur_value` updated. The
write goes **through** the priority array, never raw. (Only `w` surfaces here: `r`'s
output was consumed by the edge into `w`.)

### 3. Unknown component fails closed

```bash
post /api/v1/boards/run '{"board":{"nodes":[{"id":"x","component":"not_a_thing","config":{}}],"connections":[]}}' | jq
```

✅ `400` with `{"error":"unknown board component \`not_a_thing\`"}` — the board won't load.

### 4. Stored + versioned boards

```bash
post /api/v1/boards '{"slug":"ahu3-read","display_name":"AHU3 Read","trigger":{"kind":"manual"},"board":{"nodes":[{"id":"r","component":"read_point","config":{"point":"nube/hq/ahu-3/temp"}}],"connections":[]},"enabled":true}' | jq
post /api/v1/boards '{"slug":"ahu3-read","display_name":"AHU3 Read v2","trigger":{"kind":"manual"},"board":{...changed...},"enabled":true}' | jq  # republish → new version
curl -s "$BASE/api/v1/boards?org=nube" | jq                       # latest per slug
post /api/v1/boards/ahu3-read/run?org=nube '' | jq               # run latest
```

✅ Create returns `201` with `version:1`; republishing returns `version:2`. `GET
/boards` lists the **highest** version per slug; run-by-slug runs that version.

### 5. Scheduler — interval board (persistent scan, hot-loaded)

```bash
post /api/v1/boards '{"slug":"ahu3-guard","display_name":"AHU3 Guard","trigger":{"kind":"interval","seconds":1},"board":{"nodes":[{"id":"r","component":"read_point","config":{"point":"nube/hq/ahu-3/temp"}},{"id":"w","component":"write_point","config":{"point":"nube/hq/ahu-3/sp","priority":14}}],"connections":[{"from_node":"r","from_port":"output","to_node":"w","to_port":"value"}]},"enabled":true}'
```

✅ The board is **hot-loaded** — `create_board` registers its loop immediately, no
restart (`create.rs` → `scheduler.register`). The log shows `board loop registered`;
`slots[13]` (slot-14) populates and re-reads/re-writes each scan. A `write_point`
that re-commands the **same** value coalesces (no history spam). A disabled board is
stored but never fires. `trigger.seconds:0` is rejected at create (`400`).

### 6. Saving does not re-fire the trigger (regression gate)

Re-publish the interval board a few times (each is a new version / new row id):

```bash
post /api/v1/boards '{"slug":"pacer","display_name":"v1","trigger":{"kind":"interval","seconds":1},"board":{"nodes":[{"id":"t1","component":"trigger","config":{"every":1,"unit":"sec"}}],"connections":[]},"enabled":true}'
post /api/v1/boards '{"slug":"pacer","display_name":"v2",...same...}'   # save again
post /api/v1/boards '{"slug":"pacer","display_name":"v3",...same...}'   # and again
curl -s "$BASE/api/v1/boards/pacer/outputs?org=nube" | jq '.[]|select(.port=="count")'
```

✅ Each republish **replaces** the board's single scheduler loop (keyed by stable
`(org, site_id, slug)`), it does not add another — so the trigger's `count` keeps
climbing on one cadence instead of jumping by one extra loop per save, and `boot`
stays `false` after the first. (Covered by
`api_tests::flow::republishing_a_board_does_not_leak_a_second_loop` — `active()`
stays `1` across republishes, `0` after `PATCH enabled:false`.)

### 7. Live values — snapshot + SSE stream

```bash
curl -s "$BASE/api/v1/boards/ahu3-guard/outputs?org=nube" | jq          # snapshot
curl -N "$BASE/api/v1/boards/ahu3-guard/outputs/stream?org=nube"        # live stream (SSE)
```

✅ `/outputs` returns the latest `[{node, port, value, quality, at}]`. `/outputs/stream`
emits that snapshot on connect, then a fresh snapshot every scan/run. The flow editor
consumes the stream (real-time, last-known-good per port) instead of the old 5 s poll.

### 8. emit_spark from a rule board

```bash
post /api/v1/boards/run '{"board":{"nodes":[
  {"id":"r","component":"read_point","config":{"point":"nube/hq/ahu-3/temp"}},
  {"id":"s","component":"emit_spark","config":{"site":"nube/hq","rule":"temp-check","severity":"warning","message":"…"}}],
  "connections":[{"from_node":"r","from_port":"output","to_node":"s","to_port":"value"}]}}' | jq
curl -s "$BASE/api/v1/sparks?org=nube" | jq '.[]|select(.rule=="temp-check")'
```

✅ The board emits node `s` (`output:"flow"`); `GET /sparks` lists a new row with the
`site_id` resolved from the `site:"nube/hq"` prefix. A connected `value`/`finding`
inport overrides the `message` config.

---

## 5. Acceptance criteria ("done")

- [x] Inline `/boards/run` ticks source nodes and returns outports with `value` + `quality`.
- [x] read → write wiring commands a point through the priority array; an unchanged re-command coalesces.
- [x] Unknown component fails the load closed (`400`).
- [x] Stored boards version on republish; list/get/run resolve latest.
- [x] Interval boards run on the **persistent scan engine** (no rebuild per tick) and are **hot-loaded** on create.
- [x] **Saving an interval board does not re-fire the trigger** (one loop per board, Session clock survives).
- [x] `/outputs` snapshot and `/outputs/stream` SSE both serve `{value, quality, ts}`.
- [x] `emit_spark` records a finding (`site_id` resolved from the `site` prefix).

---

## 6. Gotchas

- **One-shot `run()` only surfaces un-consumed outports.** A node whose output feeds a
  downstream node is drained by the fan-out forwarder, so it won't appear in
  `/boards/run` outputs (it's observable at the terminal node it fed). The **persistent
  engine** sees interior links too (via the network event stream).
- **Only source nodes (no inbound connection) get ticked.** A board where every node
  has an inbound edge produces nothing — TRIAGE §8.
- **One-shot settle budget is `MAX_SETTLE=120 s`** (`board/run.rs`); a board that never
  settles (e.g. an awaited `agent_call`) is cut off there. The scan engine instead
  settles each scan in ~50 ms and is eventually-consistent (a slow node's value lands a
  scan or two late).
- `agent_call` with `await:true` blocks the one-shot run on the agent decision and
  surfaces it on `output`; `await:false` is detached. It **fails closed without an agent
  runtime** (`RUBIX_AI=0`) — a recursion guard for the agent's own `run_board` tool.
- Subscription-triggered boards need the bus; with `RUBIX_ZENOH=0` they're skipped with
  a warning. They run **one-shot per `cur` sample** (not the persistent engine).
- **Node state policy is explicit.** A new stateful node must declare a `StatePolicy`;
  picking `Ephemeral` means it resets on every save. The `trigger` uses `Session`.
- The scheduler is on by default; `RUBIX_SCHEDULER=0` disables all board loops.

## 7. Known issues / fixes (history)

- **2026-06-14 — save re-fired the trigger.** Root cause: republishing writes a new
  version *row with a new id*, but the scheduler keyed loops by `id`, so a save never
  cancelled the prior loop — every save left another loop running, each ticking the
  trigger. Fixed by keying the loop table **and** node-state scope on the stable
  `(org, site_id, slug)`; loops re-fetch the latest version by it, and the trigger's
  clock (now `Session` node state) carries across the republish. Verified via the API
  (`republishing_a_board_does_not_leak_a_second_loop`).
- **2026-06-14 — flow-runtime redesign.** Interval boards moved from rebuild-per-tick
  to the persistent `BoardEngine`; the seam became async + push-capable + quality-aware
  with a typed `FlowAccessError`; added the SSE `/outputs/stream`; disabled reflow's
  `ws://localhost:8080` tracing dial.
- **2026-06-13 — doc bodies (now baked in above):** `/boards/run` takes `{"board":…}`;
  `POST /boards` field is `board` (response `graph`) and requires `display_name` +
  `trigger` (`{"kind":…}`-tagged); a writable point is `kind:"sp"`/`"cmd"`.
- **Superseded:** the old "a newly added scheduled board needs a server restart" note is
  **no longer true** — `create_board` hot-registers the loop. Republish is also hot
  (and now correctly replaces, not duplicates, the loop).
