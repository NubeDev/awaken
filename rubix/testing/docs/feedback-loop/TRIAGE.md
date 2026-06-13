# Triage — Symptom → Root Cause

> After [CAPTURE.md](CAPTURE.md). Find your symptom, run the confirming check,
> then go to [FIX_LOOP.md](FIX_LOOP.md). Don't skip the confirming check — the
> obvious cause is often not the real one.

---

## Symptom index

| Symptom | Most likely cause | Confirm with | Section |
|---------|-------------------|--------------|---------|
| Sim log shows publishes, point `cur_value` null | keyexpr mismatch / point not provisioned | `point.json` keyexpr vs published key | [§1](#1-sim-publishing-but-no-cur-landing) |
| No sim publishes at all | driver not spawned / out-of-grant fail-closed | `server.log` spawn lines, `drivers.json` | [§2](#2-driver-not-publishing) |
| `/query` returns 503 | `RUBIX_QUERY=0` | `env.txt` | [§3](#3-503-from-a-feature-route) |
| `/agent/chat` returns 503 | `RUBIX_AI=0` (default off) | `env.txt` | [§3](#3-503-from-a-feature-route) |
| `query_count` 0 but `db_state` > 0 | query-layer bug (view/schema/scope) | compare the two artifacts | [§4](#4-data-in-store-but-query-sees-nothing) |
| Agent write didn't apply | suspended in escalation band, not committed | run status `suspended` | [§5](#5-agent-write-didnt-take) |
| Cross-site read/write succeeded | scope not enforced (or scope unset) | the keyexpr vs the run/principal scope | [§6](#6-scope-leak) |
| Bus queryable silent for a key | node doesn't own that site | which `{org}/{site}` the node holds | [§7](#7-queryable-silent) |
| Board run returns no outputs | source node didn't tick / settle budget | node graph, `server.log` | [§8](#8-board-produced-nothing) |
| 4xx on body shape | contract drift | `openapi_slice.json` | [§9](#9-contract-drift) |

---

## 1. Sim publishing but no cur landing

The sim logs `put` on `nube/hq/ahu-3/temp/cur` but `point.json` has `cur_value: null`.

- Confirm: does `point.json`'s `keyexpr` **exactly** equal the published key minus
  `/cur`? The sim publishes `{config.point}/cur`; the store records only for a
  point whose keyexpr resolves to that.
- Causes: the point/equip/site wasn't created (step 4 of QUICKSTART), or a slug
  mismatch (`ahu-3` vs `ahu3`), or `config.point` doesn't match the provisioned
  topology.
- Fix paths: provision the matching site/equip/point, or align `config.point` to
  the keyexpr that exists.

## 2. Driver not publishing

`server.log` has no driver attach line, or the sim exits immediately.

- **Spawned?** `RUBIX_ZENOH=0` disables the supervisor entirely — no drivers. Check
  `env.txt`. Also: `RUBIX_DRIVERS` path wrong, or `drivers.json` absent (→ "no
  drivers", a valid-but-silent config).
- **Malformed manifest?** A present-but-bad `drivers.json` fails closed — the
  supervisor errors. `server.log` names the parse error.
- **Out-of-grant fail-closed?** If `config.point` is **not** under a
  `capabilities.grants[].prefix`, the sim attaches but refuses every publish
  locally (named denial in its log) — *nothing* reaches the mesh. Align the grant
  to cover the point.

## 3. 503 from a feature route

Not a missing route — a disabled subsystem. `503` ⇒ check `env.txt`:
`/query`→`RUBIX_QUERY`, `/agent/chat`→`RUBIX_AI` (**default 0**), `/his/flush`→
`RUBIX_HIS_PARQUET`. Set the var and reboot.

## 4. Data in store but query sees nothing

`db_state.txt` shows rows; `query_count.json` shows 0.

- The write path is fine; the bug is in the query layer. Suspects: a stale
  `points_cur` view definition, a `PRAGMA table_info` schema mismatch, or a
  `QueryScope` filtering out everything (a scoped query whose `{org}/{site}`
  doesn't match the rows). Run the same SQL unscoped to isolate.

## 5. Agent write didn't take

The agent "wrote" but `point.json` is unchanged.

- Confirm the run status: `GET /api/v1/runs?status=suspended`. A write in the
  **escalation band** (`RUBIX_AI_ESCALATION_FLOOR ≤ priority < RUBIX_AI_MIN_PRIORITY`)
  **suspends with the store untouched** — this is correct behavior, not a bug. It
  applies only on `POST /api/v1/runs/{id}/resume`. Below the floor it's denied
  outright.

## 6. Scope leak

A scoped run or principal reached a keyexpr outside its `{org}/{site}`.

- Confirm: is a scope actually set? An **unscoped** run/edge-no-auth request is
  *global* by design. The leak is real only if a scope was bound and `covers()`
  let a sibling through (e.g. `nube/hq` reaching `nube/hq2`). That's a
  `Capability::covers` / `TenantScope::covers` bug — check the path-boundary logic.

## 7. Queryable silent

A `**/write` or `**/his/**` query gets no reply from a node.

- By design: a node answers only for keys under a `{org}/{site}` **it owns** (has a
  site row for). If the site lives on another node, silence is correct. Confirm the
  owning node has the site provisioned.

## 8. Board produced nothing

`/boards/run` returns `[]`.

- Source nodes are those with **no inbound connection**; only they get ticked. A
  board whose every node has an inbound edge has no source → nothing fires. Or the
  graph didn't settle within `MAX_SETTLE` (120s). Check the graph shape and
  `server.log`. An unknown `component` fails the load closed (named error).

## 9. Contract drift

A request 4xx's on body shape, or a documented path is gone.

- `curl -s $BASE/api-docs/openapi.json | jq '.paths["<path>"]'`. The doc is stale
  or the request is wrong. Fix the cheatsheet/feature doc, bump its `Verified:`
  line. If the path is genuinely gone, that's a real backend change — check recent
  commits / `docs/sessions/WS-xx`.
