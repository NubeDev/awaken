# Feature — Reflow Boards

> Verified: **verified** live on `rubix-gaps` (`bdbd01f7`, 2026-06-13). All six
> gates green; backend correct on every one. Fixed 4 doc bugs in the request bodies
> (see "Known issues / fixes"). Source: `rubix-flow/src/board/`,
> `rubix-flow/src/node/`, `rubix-server/src/{api/boards,scheduler}`.

Covers: the reflow node palette, board JSON shape, inline `/boards/run`, stored +
versioned boards, and the scheduler (interval + `cur`-subscription triggers).

Prereq: stack up. For `agent_call` boards, `RUBIX_AI=1`. `$BASE`, `post()` from the
cheatsheet.

---

## The node palette (6 built-ins)

`rubix-flow/src/board/registry.rs:14` (`COMPONENTS`). Each node's ports/config
(`src/node/`):

| component | inports | outports | config | does |
|-----------|---------|----------|--------|------|
| `read_point` | `trigger` | `output`, `error` | `point` (keyexpr) | reads current value |
| `write_point` | `value` | `output`, `error` | `point`, `priority` (1..16, default 16) | commands through the priority array |
| `query_his` | `trigger` | `output`, `error` | `point`, `limit` (default 100) | recent history as JSON array |
| `emit_spark` | `value` | `output`, `error` | `site`, `rule`, `severity` (default warning), `message` | records a finding (+ bus publish) |
| `agent_call` | `value` | `output`, `error` | `prompt`, `thread`, `await` (default false) | raises an embedded-agent run |
| `trigger` | `trigger` | `boot`, `count`, `output`, `error` | `every`, `unit` | self-paced timing source; fires only when its period elapsed |

The `trigger` node is a **source** (its inport has no inbound edge — the scheduler
ticks it) but owns its own cadence: it fires only when `every × unit` has elapsed
since its last fire, regardless of tick rate. On a fire it emits `boot` (`true`
only on the first fire after server start), `count` (total fires), and `output` (a
level that toggles each fire). Between fires it emits nothing, so the board settles.
Period state is keyed by node id in `src/node/trigger/trigger_state.rs` because each
scheduler tick rebuilds the actor fresh.

A board graph is `{ "nodes": [{id, component, config}], "connections": [{from_node, from_port, to_node, to_port}] }`.

**Two wire shapes wrap that graph** (both confirmed live — this is where the
scaffold's bodies were wrong):
- `POST /boards/run` takes `{"board": <graph>}` (the graph is **not** posted bare).
- `POST /boards` takes `{slug, display_name, trigger, board: <graph>, enabled?}` —
  `display_name` and `trigger` are **required**; the graph field is `board` (the
  response calls it `graph`). `trigger` is tagged: `{"kind":"manual"}`,
  `{"kind":"interval","seconds":N}` (N≥1), or `{"kind":"subscription","key":"…"}`.
- Both run endpoints reply `{"outputs":[{node, port, value}]}`.

---

## Runbook

### 1. Inline run — read a point

```bash
post /api/v1/boards/run '{"board":{"nodes":[{"id":"r","component":"read_point","config":{"point":"nube/hq/ahu-3/temp"}}],"connections":[]}}' | jq
```

✅ Returns `{"outputs":[{"node":"r","port":"output","value":<cur>}]}` — node `r`'s
current value. (Node `r` has no inbound connection ⇒ it's a **source** ⇒ it ticks.)

### 2. Inline run — read → write through the priority array

The writable point must be created with `kind:"sp"` (or `"cmd"`) — `"setpoint"` is
**not** a valid kind (`PointKind` is `sensor`/`cmd`/`sp`). Provision `nube/hq/ahu-3/sp`
as `kind:"sp"` first.

```bash
post /api/v1/boards/run '{"board":{
  "nodes":[
    {"id":"r","component":"read_point","config":{"point":"nube/hq/ahu-3/temp"}},
    {"id":"w","component":"write_point","config":{"point":"nube/hq/ahu-3/sp","priority":13}}
  ],
  "connections":[{"from_node":"r","from_port":"output","to_node":"w","to_port":"value"}]
}}' | jq
```

✅ Outputs carry node `w` on port `output` with the written value; `GET
/api/v1/points/$SP` shows `priority_array.slots[12]` (slot-13) == that value and
`cur_value` updated. The write goes **through** the priority array — never raw. (Only
`w` appears in outputs here: `r`'s output was consumed by the edge into `w`. A source
whose output is *not* consumed — e.g. the v2 board in step 4 — does surface.)

### 3. Unknown component fails closed

```bash
post /api/v1/boards/run '{"board":{"nodes":[{"id":"x","component":"not_a_thing","config":{}}],"connections":[]}}' | jq
```

✅ `400` with `{"error":"unknown board component \`not_a_thing\`"}` — the board won't load.

### 4. Stored + versioned boards

```bash
post /api/v1/boards '{"slug":"ahu3-read","display_name":"AHU3 Read","trigger":{"kind":"manual"},"board":{"nodes":[{"id":"r","component":"read_point","config":{"point":"nube/hq/ahu-3/temp"}}],"connections":[]},"enabled":true}' | jq
post /api/v1/boards '{"slug":"ahu3-read","display_name":"AHU3 Read v2","trigger":{"kind":"manual"},"board":{...changed...},"enabled":true}' | jq  # republish → new version
curl -s $BASE/api/v1/boards | jq                       # latest per slug
post /api/v1/boards/ahu3-read/run '' | jq              # run latest
```

✅ Create returns `201` with `version:1`; republishing the same slug returns
`version:2`. `GET /boards` lists the **highest** version per slug; run-by-slug runs
that version and returns its node outputs.

### 5. Scheduler (interval trigger)

Store a board with an interval trigger and `enabled:true` (default `RUBIX_SCHEDULER=1`):

```bash
post /api/v1/boards '{"slug":"ahu3-guard","display_name":"AHU3 Guard","trigger":{"kind":"interval","seconds":1},"board":{"nodes":[{"id":"r","component":"read_point","config":{"point":"nube/hq/ahu-3/temp"}},{"id":"w","component":"write_point","config":{"point":"nube/hq/ahu-3/sp","priority":14}}],"connections":[{"from_node":"r","from_port":"output","to_node":"w","to_port":"value"}]},"enabled":true}'
```

> ⚠️ A **newly added** scheduled board is **not** hot-loaded — the running scheduler
> launches its board set at boot (`create.rs` says so explicitly, and a *republish*
> of an already-scheduled board *is* hot, but a brand-new slug is not). **Restart the
> server** to have the scheduler pick up a new interval board. `trigger.seconds:0` is
> rejected at create (`400`).

✅ After restart the log shows `board scheduler launched boards=1` (only the enabled
board — a `enabled:false` board is excluded) and `scheduled board ran
board="ahu3-guard"` every second; `slots[13]` (slot-14) populates and changes each
tick (re-reads live temp, re-writes). A disabled board is stored but never fires.

### 6. emit_spark from a rule board

A board with an `emit_spark` node records a finding and (bus up) publishes it on
`{org}/{site}/spark/{rule}/{id}` — see [ZENOH_BUS.md](ZENOH_BUS.md) §5 and the
dispatch loop in [AI_TOOLS_AND_AGENT.md](AI_TOOLS_AND_AGENT.md).

```bash
post /api/v1/boards/run '{"board":{"nodes":[
  {"id":"r","component":"read_point","config":{"point":"nube/hq/ahu-3/temp"}},
  {"id":"s","component":"emit_spark","config":{"site":"nube/hq","rule":"temp-check","severity":"warning","message":"…"}}],
  "connections":[{"from_node":"r","from_port":"output","to_node":"s","to_port":"value"}]}}' | jq
curl -s $BASE/api/v1/sparks | jq '.[]|select(.rule=="temp-check")'
```

✅ The board emits node `s` (`output:"flow"`); `GET /api/v1/sparks` lists a new row
with `rule:"temp-check"`, `severity:"warning"`, and `site_id` resolved from the
`site:"nube/hq"` prefix to the HQ site UUID. Note: the spark model field is
**`site_id` (UUID)**, not a `site` string — a connected `value` inport **overrides**
the `message` config (so the message becomes the upstream value rendered as text).

---

## Acceptance criteria ("done")

- [x] Inline `/boards/run` ticks source nodes and returns every *un-consumed* outport.
- [x] read → write wiring commands a point through the priority array (slot-13).
- [x] Unknown component fails the load closed (`400`).
- [x] Stored boards version on republish; list/get/run resolve latest.
- [x] Scheduler fires interval boards; disabled never fires; new board needs restart.
- [x] `emit_spark` records a finding (`site_id` resolved from the `site` prefix).

---

## Gotchas

- **Only source nodes (no inbound connection) get ticked.** A board where every
  node has an inbound edge produces nothing — TRIAGE §8.
- Settle budget is `MAX_SETTLE=120s` (`board/run.rs`); a board that never settles
  is cut off there.
- `agent_call` with `await:true` blocks the single-shot run on the agent decision
  and surfaces it on `output` (a downstream node can branch); `await:false` is
  detached (the node acknowledges, the run proceeds out-of-band). It **fails closed
  without an agent runtime** (`RUBIX_AI=0`) — that's a recursion guard for the
  agent's own `run_board` tool.
- Subscription-triggered boards need the bus; with `RUBIX_ZENOH=0` they're skipped
  with a warning.
- **Remaining (not a bug):** the running scheduler isn't hot-reconfigured — a board
  *added* after boot is picked up on the next restart (republish of an existing
  scheduled board *is* hot). No zenoh board deploy to stations yet.

## Known issues / fixes

Verified live 2026-06-13 (`bdbd01f7`, `RUBIX_ZENOH=1`, sim publishing `temp`). All
six gates green; **backend correct on every one — every failure was a doc bug** in
the request bodies, fixed above:

1. `/boards/run` and `/boards/{slug}/run` take `{"board": <graph>}`, not the bare
   graph (`RunBoardRequest.board`). The scaffold posted the graph bare → parse error.
2. `POST /boards` graph field is `board`, not `graph` (`CreateBoard.board`); the
   response calls it `graph` (`BoardView.graph`).
3. `POST /boards` requires `display_name` and `trigger` (no defaults). `trigger` is
   internally tagged `{"kind":"…"}` — the scaffold's `{"interval":{"seconds":1}}`
   was wrong; correct is `{"kind":"interval","seconds":1}`.
4. The writable point is `kind:"sp"` (or `"cmd"`); `"setpoint"` is not a valid
   `PointKind` and returns `422`.

Behavioral notes confirmed (not bugs): only outports whose value isn't consumed by a
downstream edge surface in `outputs`; a newly-added scheduled slug is picked up on
the next scheduler launch (restart), not hot — matches the `create.rs` doc-comment
and the doc's "Remaining" note; `emit_spark` stores `site_id` (resolved from the
`site` prefix) and lets a `value` inport override the `message` config.
