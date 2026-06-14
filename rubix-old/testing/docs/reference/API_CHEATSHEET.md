# API Cheatsheet — curl-Ready

> Verified: code-grounded on `rubix-gaps` tip, 2026-06-13. **`/api-docs/openapi.json`
> is the source of truth** for request/response shapes — when a snippet fails,
> fetch it and fix here. Routes from `crates/rubix-server/src/api/mod.rs`.

**No auth on the edge profile by default.** Unlike nexus there is no cookie/CSRF
flow. If `RUBIX_OIDC_ISSUER` is set (or you're on cloud), add `-H "authorization:
Bearer $TOKEN"` to every call; mint a PAT via `POST /api/v1/tokens`. Otherwise
calls go through unauthenticated.

```bash
export BASE=http://127.0.0.1:8088   # the `make dev-be` default; raw `cargo run` uses :8080
post()  { curl -s -X POST "$BASE$1" -H content-type:application/json -d "$2"; }
patch() { curl -s -X PATCH "$BASE$1" -H content-type:application/json -d "$2"; }
del()   { curl -s -X DELETE "$BASE$1"; }
```

---

## Meta (public, no auth even when auth is on)

```bash
curl -s $BASE/healthz | jq                  # → {"status":"ok","version":"0.1.0"}
curl -s $BASE/api-docs/openapi.json | jq '.info, (.paths|keys)'   # contract truth
```

---

## Sites / equips / points

```bash
SITE=$(post /api/v1/sites '{"org":"nube","slug":"hq","display_name":"HQ"}' | jq -r .id)
curl -s $BASE/api/v1/sites | jq                                  # list (tag filters supported)
curl -s $BASE/api/v1/sites/$SITE | jq

EQUIP=$(post /api/v1/equips "$(jq -nc --arg s "$SITE" '{site_id:$s,path:"ahu-3",display_name:"AHU 3"}')" | jq -r .id)
# `display_name` is REQUIRED on CreatePoint/CreateEquip/CreateSite (omit → 422).
# Point create+GET wrap as {keyexpr, point:{…}} → id is `.point.id` (sites/equips: `.id`).
POINT=$(post /api/v1/points "$(jq -nc --arg e "$EQUIP" '{equip_id:$e,slug:"sp",display_name:"Setpoint",kind:"sp",unit:"°C"}')" | jq -r .point.id)
curl -s $BASE/api/v1/points/$POINT | jq '{keyexpr, cur_value:.point.cur_value, pa:.point.priority_array}'
```

`kind` is `sensor` | `cmd` | `sp`. Only `cmd`/`sp` are writable; `/cur` ingest is
**sensor-only** (writable points 400 it, sensors 400 `/write`). Every kind serializes
a 16-slot `priority_array` — read-only-ness is enforced at the write endpoint, not by
array absence. `keyexpr` returns `{org}/{site}/{equip-path}/{point}`.

---

## Priority-array command path

```bash
# write at a slot (1..=16, lower wins). Returns the new effective value.
post /api/v1/points/$POINT/write '{"priority":8,"value":21.0}' | jq

# relinquish a slot (frees it; effective value falls back to the next-lowest level)
del /api/v1/points/$POINT/write/8 | jq

# ingest a sensor reading (sets cur_value, appends history)
post /api/v1/points/$POINT/cur '{"value":21.7}' | jq
```

A write below the agent ceiling is fine over HTTP — the ceiling only gates the
**agent**, not direct operators. See AI_TOOLS for the gated path.

---

## History

```bash
curl -s "$BASE/api/v1/points/$POINT/his?limit=50" | jq           # recent samples
post /api/v1/points/$POINT/his '{"ts":"2026-06-13T00:00:00Z","value":20.0}'   # insert
post /api/v1/his/rollup '{"points":["'$POINT'"],"interval":"hour","agg":"avg"}' | jq
# /api/v1/his/flush ages SQLite rows to Parquet — 503 unless RUBIX_HIS_PARQUET set
```

Rollup `interval`: `minute|five_minute|fifteen_minute|hour|day|week`.
`agg`: `avg|min|max|sum|count|first|last`. Optional `start`/`end` RFC-3339 bounds.

---

## Query (READ-ONLY DataFusion over SQLite)

```bash
post /api/v1/query '{"sql":"SELECT count(*) AS n FROM his"}' | jq
post /api/v1/query '{"sql":"SELECT keyexpr, cur_value FROM points_cur"}' | jq
# 503 if RUBIX_QUERY=0. Only SELECT/WITH; one statement; no DDL/DML.
```

Canonical tables: `sites`, `equips`, `points`, `his`, `sparks`, plus the
`points_cur` view (flattened effective value + keyexpr).

---

## Sparks (rule findings)

```bash
post /api/v1/sparks '{"site_id":"'$SITE'","rule":"simultaneous-heat-cool","severity":"warning","message":"…","point_ids":["'$POINT'"]}' | jq
curl -s $BASE/api/v1/sparks | jq
post /api/v1/sparks/$SPARK/ack ''                  # acknowledge
```

Creating a spark publishes it on `{org}/{site}/spark/{rule}/{id}` when the bus is
up — which is what the agent dispatcher subscribes to.

---

## Boards (reflow)  — see [features/BOARDS_REFLOW.md](../features/BOARDS_REFLOW.md)

The graph is wrapped in `{"board": <graph>}` on `/run`; `POST /boards` takes the
graph under `board` (required `display_name` + `trigger`), and the response calls it
`graph`. Outputs carry `{node, port, value, quality}`. Board endpoints are
tenant-scoped: `?org=` required, `?site_id=` optional.

```bash
# the node palette (ports + config schema for all 8 components)
curl -s $BASE/api/v1/boards/components | jq

# run an inline board once → {"outputs":[{node,port,value,quality}]}
post /api/v1/boards/run '{"board":{"nodes":[{"id":"r","component":"read_point","config":{"point":"nube/hq/ahu-3/temp"}}],"connections":[]}}' | jq

# stored + versioned (trigger is {"kind":"manual"|"interval"|"subscription", …})
post /api/v1/boards '{"slug":"ahu3-guard","display_name":"AHU3 Guard","trigger":{"kind":"interval","seconds":1},"board":{...},"enabled":true}' | jq
curl -s "$BASE/api/v1/boards?org=nube" | jq                 # list (latest per slug)
curl -s "$BASE/api/v1/boards/ahu3-guard?org=nube" | jq
post '/api/v1/boards/ahu3-guard/run?org=nube' ''            # run latest stored
patch /api/v1/boards/ahu3-guard?org=nube '{"enabled":false}'  # enable/disable (un)registers the loop
del '/api/v1/boards/ahu3-guard?org=nube'

# live values: snapshot, and the SSE stream the editor consumes
curl -s "$BASE/api/v1/boards/ahu3-guard/outputs?org=nube" | jq        # [{node,port,value,quality,at}]
curl -N "$BASE/api/v1/boards/ahu3-guard/outputs/stream?org=nube"      # SSE: snapshot then deltas
```

Components: `read_point`, `write_point`, `query_his`, `emit_spark`, `agent_call`.
See [../features/BOARDS_REFLOW.md](../features/BOARDS_REFLOW.md) for node ports/config.

---

## Agent + runs (RUBIX_AI=1)

```bash
post /api/v1/agent/chat '{"message":"what is the current temp on ahu-3?"}' | jq
# 503 if RUBIX_AI=0 (default). Escalation-band write → {status:"awaiting_approval", run_id}

curl -s "$BASE/api/v1/runs?status=suspended" | jq  # operator surface
curl -s $BASE/api/v1/runs/$RUN | jq
post /api/v1/runs/$RUN/resume ''                    # re-apply held write (gating re-checked, one-shot)
post /api/v1/runs/$RUN/cancel ''                    # drop, store untouched
```

---

## Tokens (PAT) + MCP + widgets

```bash
post /api/v1/tokens '{"role":"service","org":"nube","site":"hq"}' | jq   # mint a PAT (auth must be on)
curl -s $BASE/api/v1/tokens | jq ; del /api/v1/tokens/$TOK
post /api/v1/mcp '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' | jq   # JSON-RPC into scoped tools
curl -s $BASE/api/v1/widgets | jq                                        # agent-pinned tiles
```

---

## When a snippet 4xx/5xx's

1. `curl -s $BASE/api-docs/openapi.json | jq '.paths["<path>"]'` — confirm method
   + body schema.
2. Fix the snippet here, bump the `Verified:` line.
3. A `503` means the subsystem is disabled (`RUBIX_QUERY`/`RUBIX_AI`/`RUBIX_HIS_PARQUET`),
   not a missing route. A gone/renamed path is real drift — check recent commits.
