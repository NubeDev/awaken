# Feature — AI Tools, Agent, HITL, Dispatch & MCP

> Verified: **verified** live on `rubix-gaps` (`bdbd01f7`, 2026-06-13) against a real
> OpenAI run (`provider=openai`, `model=gpt-4o-mini`). All eight gates green; backend
> correct on every safety property. Fixed 2 doc bugs (`thread_id` required on chat;
> `?origin=` filter doesn't exist). Source: `rubix-tools/src/`,
> `rubix-server/src/{agent,dispatch,api/runs,mcp}`.

Covers: the four BMS tools, the three-band write gating, the runs/HITL surface,
spark→agent dispatch, and the MCP adapter. This is where rubix becomes agentic —
and where the safety invariants matter most.

Prereq: stack up with `RUBIX_AI=1`. The genai provider reads its key at run time:
`RUBIX_AI_PROVIDER` defaults to `openai` and `RUBIX_AI_MODEL_ID` to `gpt-4o-mini`, so
the openai provider reads `OPENAI_API_KEY` from the env (never put a key in
`drivers.json`, a board, or any doc — pass it as a process env var only). Or drive the
loop with the offline scripted-executor test path (no key). `$BASE`, `post()` from the
cheatsheet.

> **Every `/agent/chat` body needs a `thread_id`** (`ChatRequest` has `thread_id`
> *and* `message`, both required) — the scaffold omitted it and would `422`. The chat
> response is `{response, steps, status, run_id?}` where `status` is `completed` or
> `awaiting_approval`.

---

## The tools (`TypedTool` over `PointAccess`)

| tool | gating | does |
|------|--------|------|
| `read_point` | none (read-only) | current value by keyexpr |
| `write_point` | **three-band priority gate** (below) | command through the priority array |
| `query` | read-only `SELECT`/`WITH`, single statement | tenant-scoped SQL |
| `run_board` | board writes re-gated as `write_point` | evaluate a board once |
| `pin_widget` | scope | pin a dashboard tile |

### Three-band write gating (the core safety property)

`rubix-tools/src/tool/write_point.rs`. Given `agent_min_priority`
(`RUBIX_AI_MIN_PRIORITY`, default 13) and `escalation_floor`
(`RUBIX_AI_ESCALATION_FLOOR`, default 1):

| band | condition | result |
|------|-----------|--------|
| commit | `priority >= agent_min_priority` (≥13) | write applies immediately |
| **escalate** | `floor <= priority < ceiling` (1..12) | **suspends** with a `SuspendTicket`; store untouched |
| deny | `priority < floor` | `ToolError::Denied`; never reachable, even with approval |

Default priority when the agent omits it: **16** (lowest authority).

---

## Runbook

### 1. Read via the agent

```bash
post /api/v1/agent/chat '{"thread_id":"t-read","message":"what is the current temperature on ahu-3? Point keyexpr nube/hq/ahu-3/temp."}' | jq
```

✅ The run calls `read_point` and answers with the live `cur_value` (e.g. `steps:2`,
`status:"completed"`). (`503` here ⇒ `RUBIX_AI=0`.)

### 2. Commit-band write

Prompt the agent to set a point at/below the ceiling (priority ≥ 13):

```bash
post /api/v1/agent/chat '{"thread_id":"t-commit","message":"Call write_point with point=nube/hq/ahu-3/sp, value=22, priority=13. Do it once."}' | jq
```

✅ The write applies; `GET /api/v1/points/$SP` shows `slots[12]` (slot-13) == the value
and `cur_value` updated; the run completes. **Be explicit** — a small model
(`gpt-4o-mini`) will thrash on a vague prompt and burn its rounds without a clean
write (that's model variance, not a backend bug — the deterministic MCP path below
proves the tool commits). To prove the tool independent of the model:
`POST /api/v1/mcp '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"write_point","arguments":{"point":"nube/hq/ahu-3/sp","value":20,"priority":13}}}'`
→ `structuredContent.effective == 20`.

### 3. Escalation-band write suspends (HITL)

Prompt a write at priority 8 (operator authority, above the agent's ceiling):

```bash
post /api/v1/agent/chat '{"thread_id":"t-esc","message":"Call write_point with point=nube/hq/ahu-3/sp, value=18, priority=8. Once."}' | jq
# → {"status":"awaiting_approval","run_id":"…","steps":1}
curl -s "$BASE/api/v1/runs?status=suspended" | jq '[.[]|{id,status,origin,pending_write}]'
```

✅ Response is `awaiting_approval` + a `run_id`; the run persists `suspended` with
the held write attached; **the store is untouched** (`GET .../points/$PT` unchanged).

### 4. Resume / cancel

```bash
curl -s -X POST $BASE/api/v1/runs/$RUN/resume | jq   # {run_id, point, priority, effective}
curl -s $BASE/api/v1/points/$SP | jq '.point.priority_array.slots[7]'   # slot-8 now reflects the write
curl -s -o /dev/null -w '%{http_code}\n' -X POST $BASE/api/v1/runs/$RUN/resume   # → 409 (settled run)
```

✅ Resume applies the held write through the priority array (`effective` returned;
slot-8 populates and the lower slot number wins `cur_value` over any slot-13 commit)
and is **one-shot** — a second resume is `409` "is `resumed`, not suspended". The
gate is **re-checked at approval** (a floor raised between suspend and approve closes
the write with `403`). `POST .../cancel` instead drops it (store untouched, `cur`
unchanged); resuming a cancelled run is also `409`.

### 5. Deny band

With `RUBIX_AI_ESCALATION_FLOOR=5` (requires a restart — the floor is read at boot),
a write at priority 3 (below the floor). Deterministic via MCP:

```bash
post /api/v1/mcp '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"write_point","arguments":{"point":"nube/hq/ahu-3/sp","value":10,"priority":3}}}' | jq '.result'
```

✅ `isError:true`, text `"Denied: priority 3 is operator-reserved (below escalation
floor 5)…"` — never suspends (no new `runs` row), never applies (slot null). Boundary
check (floor=5, ceiling=13): p4→deny, p5→suspend, p12→suspend, p13→commit.
Operator-reserved top slots an agent may never command.

### 6. Spark dispatch (job, not chat)

With bus + agent up (`RUBIX_AI_DISPATCH=1`), create a spark (or have a rule board
emit one):

```bash
post /api/v1/sparks '{"site_id":"'$SITE'","rule":"simultaneous-heat-cool","severity":"fault","message":"AHU-3 heating and cooling","point_ids":["'$POINT'"]}'
sleep 12   # the dispatched run executes async (up to RUBIX_AI_MAX_ROUNDS)
curl -s "$BASE/api/v1/runs" | jq '[.[]|select(.origin=="dispatch")]'   # filter client-side
```

> `GET /runs` only supports `?status=` — there is **no `?origin=` query param**
> (`ListRunsQuery` has just `status`). Filter by origin client-side, as above.

✅ `POST /sparks` publishes the finding on the bus (`{org}/{site}/spark/{rule}/…`); the
dispatcher (subscribed to `**/spark/**`) activates an agent **run** on a spark-keyed
thread (`thread_id:"spark-<spark-id>"`) with an investigate-then-act-within-gating
prompt. The run appears under `/api/v1/runs` with `origin:"dispatch"`; its tool calls
hit the same gated tools. This closes the board → spark → bus → agent loop. (The run
executes async — list after it settles, else you race its completion.)

### 7. MCP adapter

```bash
post /api/v1/mcp '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' | jq
post /api/v1/mcp '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"read_point","arguments":{"point":"nube/hq/ahu-3/temp"}}}' | jq
```

✅ JSON-RPC 2.0 (`initialize`/`tools/list`/`tools/call`) dispatches into the **same**
scoped tool registry — identical priority gating, tenant scope, and HITL escalation
(an escalation-band `write_point` over MCP suspends to a `runs` row with
`origin: mcp` and resumes through the same operator surface).

### 8. Tenant scope at the tool boundary

A run carries the `{org}/{site}` it acts within (chat: principal's site; dispatch:
spark's site, fail-closed if unknown). Tools refuse any keyexpr outside it.

✅ A run scoped to `nube/hq` reading/writing `nube/hq2/...` is refused **at the tool
boundary**, not just at HTTP; the `query` tool runs through a tenant-filtered
DataFusion session so its ad-hoc SQL reads only its own `{org}/{site}`.

---

## Acceptance criteria ("done")

- [x] `read_point` answers with live value (`steps:2`, `completed`).
- [x] Commit-band write (≥ceiling) applies (slot-13; MCP-deterministic, chat with explicit prompt).
- [x] Escalation-band write suspends, store untouched, persists to `runs` (`pending_write` held).
- [x] Resume applies the write (one-shot, 409 on re-resume); cancel drops it (store untouched).
- [x] Below-floor write denied outright; boundaries p4/p5/p12/p13 exact.
- [x] Spark → dispatch → agent run, `origin:"dispatch"` on a `spark-<id>` thread.
- [x] MCP `initialize`/`tools/list`/`tools/call` hit the same gated/scoped registry.
- [x] Scoped run refused outside its `{org}/{site}` at the tool boundary (unit + dispatch fail-closed).

---

## Gotchas

- **`RUBIX_AI` defaults to 0** — `/agent/chat` is 503 until you turn it on.
- **The escalation suspend is correct behavior, not a failed write** — TRIAGE §5.
- **Unscoped = global.** On edge with no auth, a run has no principal scope and can
  reach everything. The scope test only means something when a scope is bound.
- The genai key is read at run time — the node boots with `RUBIX_AI=1` and no key,
  and only errors on the first model call. The offline integration tests drive the
  loop with a scripted executor, no key needed.
- **Remaining (not a bug):** A2A and AG-UI outbound adapters aren't present; MCP is
  the only outbound adapter shipped.

## Known issues / fixes

Verified live 2026-06-13 (`bdbd01f7`) against a real OpenAI run (`openai` /
`gpt-4o-mini`), `RUBIX_AI=1`. **All eight gates green; backend correct on every safety
property** — the only failures were doc bugs and one model-behavior artifact.

**Doc bugs fixed (not backend bugs):**
1. `/agent/chat` requires `thread_id` (the scaffold's `{"message":…}` would `422`).
2. `GET /runs` has **no `?origin=` filter** — only `?status=`. The scaffold's
   `?origin=dispatch` silently returns all runs; filter origin client-side.

**Model-variance artifact (not a backend bug):** the first commit-band chat (Gate 2)
with a vague prompt made `gpt-4o-mini` thrash — one `write_point` call `Failed`, then
it looped `read_point`/`query` until it hit `RUBIX_AI_MAX_ROUNDS` (8) and gave up with
an empty response, slot-13 untouched. The **tool itself commits fine**: the same write
via MCP at priority 13 returned `effective:20` and landed in slot-13, and a re-run of
the chat with an explicit "call write_point with point=…, value=…, priority=13, once"
succeeded (`steps:2`). Lesson recorded in step 2: prompt small models explicitly, and
isolate tool-vs-model with the MCP path. (Also logged once during the thrash:
`invalid lifecycle transition — skipping update from=Done to=Running` — benign here,
the run still settled `completed`; worth a look if it recurs on a clean single write.)

**Safety properties confirmed exact:** three-band gating boundaries (floor=5,
ceiling=13) — p4 deny / p5 escalate / p12 escalate / p13 commit; deny never creates a
`runs` row and never writes; escalation holds `pending_write` with the store
untouched; resume is atomic one-shot (409 on re-resume) and re-checks the floor (403
if raised); cancel drops the write; dispatch confines the run to the spark's
`{org}/{site}` and **fails closed** (skips, never runs unscoped) if the site can't
resolve; the tool boundary refuses out-of-scope keys before the inner store
(`rubix-tools` `scoped::point::tests::{in_scope_calls_reach_the_inner_access,
out_of_scope_calls_are_refused_before_the_inner_access}`).

**Environment blocker (separate from this feature — see TODO note):** `cargo
run`/`cargo build` over the workspace currently fails to resolve — the untracked
`crates/rubix-datasource/` (listed as a committed workspace member in `Cargo.toml` but
itself never committed) pulls `sqlx-sqlite`'s `libsqlite3-sys 0.30` while
`rubix-server` (via `rusqlite 0.37`) needs `0.35`, and only one crate may link
`sqlite3`. `rubix-datasource` is **not** a dependency of `rubix-server`/`rubix-query`,
so the existing `target/debug/rubix` binary (pre-dating the crate) is complete and was
used for this verification; `cargo test -p <crate> --offline` also sidesteps the
re-resolve. This needs an owner decision (commit the crate with a compatible
`libsqlite3-sys`, or drop it from `members`) before `make build-be` is green again.
