# Quickstart — Stack Up, Live Data Flowing, in One Pass

> Verified: **run live on `rubix-gaps`, 2026-06-13** — full path confirmed end to
> end (build → sim spawn → `cur` lands → history accumulates). One backend bug found
> and fixed en route (inbound `cur` subscriber — see
> [../features/ZENOH_BUS.md](../features/ZENOH_BUS.md) "Known issues"). Commands
> assume CWD `rubix/`.

Goal: from a clean checkout to **a simulated `cur` value landing in the store
over zenoh**, with the API healthy. ~3 minutes. Every step has a ✅ check — do not
proceed past a failed check; go to [../feedback-loop/TRIAGE.md](../feedback-loop/TRIAGE.md).

Default port via `make`: API `8088` (`make` exports `RUBIX_ADDR=127.0.0.1:8088` so
the UI proxy reaches it). The bare `rubix` binary's own default is `0.0.0.0:8080`,
so a raw `cargo run` without `make` lands on `8080` — this runbook uses `make` and
`:8088`. Zenoh runs **peer mode** by default — a single node needs no router.
SQLite is a file (`rubix.db`), no DB server.

---

## 0. Prereqs

- Rust toolchain (workspace pins `rust-version = "1.85"`), `curl`, `jq`. (`pnpm`
  only if you also want the UI — `make dev` / `make build-ui`.)
- No Docker, no Postgres, no broker required for the edge path.

✅ `cargo --version` prints, `jq --version` prints.

---

## 1. Build

```bash
make build-be               # cargo build — all 7 crates (server bin `rubix` + sim + libs)
```

✅ `make build-be` exits 0. (First build is slow — zenoh + datafusion + reflow.)
`make build` also builds the UI; `make build-ui` builds just the UI.

---

## 2. Point the supervisor at the sim driver

The server spawns drivers listed in `RUBIX_DRIVERS` (default `drivers.json`). Write
one that grants the sim a keyexpr prefix and tells it which point to publish on.
The manifest shape is the `DriverManifest` JSON (identity + capabilities + config):

```bash
SIM_BIN=$(pwd)/target/debug/rubix-driver-sim
cat > drivers.json <<JSON
[
  {
    "identity": {
      "name": "sim",
      "protocol": "sim",
      "version": "0.1.0",
      "launch": { "command": "$SIM_BIN", "args": [] }
    },
    "capabilities": { "grants": [ { "prefix": "nube/hq/ahu-3", "access": "publish" } ] },
    "config": { "point": "nube/hq/ahu-3/temp", "period_secs": 2, "baseline": 21.0, "amplitude": 2.0 }
  }
]
JSON
```

> The `config.point` (`nube/hq/ahu-3/temp`) **must** sit under a granted `prefix`
> (`nube/hq/ahu-3`) or the sim fails closed at startup and never publishes. The
> driver publishes on `{point}/cur` → `nube/hq/ahu-3/temp/cur`.

✅ `drivers.json` exists; `config.point` is under a `capabilities.grants[].prefix`.

---

## 3. Boot the server

```bash
export BASE=http://127.0.0.1:8088
make dev-be                 # cargo run --bin rubix, binds RUBIX_ADDR=127.0.0.1:8088
# (raw equivalent: RUST_LOG=info,tower_http=debug cargo run --bin rubix)
# (make dev-ui runs the UI on :5180; make dev runs both together)
```

Leave it running; use another terminal for the rest. At boot it: opens SQLite
(`rubix.db`), opens a zenoh peer session (`RUBIX_ZENOH=1`), spawns drivers from
`drivers.json`, starts the query engine (`RUBIX_QUERY=1`) and scheduler. The agent
is **off** by default (`RUBIX_AI=0`).

```bash
curl -s $BASE/healthz | jq      # → {"status":"ok","version":"0.1.0"}
```

✅ `/healthz` returns `200` with `status:"ok"`. The server log shows
`rubix server listening` and a line for the spawned `sim` driver attaching
(liveliness token).

---

## 4. Provision the site/equip/point the sim publishes for

The bus only answers (and the store only records history) for a point that
exists. Create the matching `nube/hq/ahu-3/temp` topology over HTTP:

```bash
post() { curl -s -X POST "$BASE$1" -H content-type:application/json -d "$2"; }

SITE=$(post /api/v1/sites '{"org":"nube","slug":"hq","display_name":"HQ"}' | jq -r .id)
EQUIP=$(post /api/v1/equips "$(jq -nc --arg s "$SITE" '{site_id:$s,path:"ahu-3",display_name:"AHU 3"}')" | jq -r .id)
POINT=$(post /api/v1/points "$(jq -nc --arg e "$EQUIP" '{equip_id:$e,slug:"temp",display_name:"Temp",kind:"sensor",unit:"°C"}')" | jq -r .point.id)
echo "site=$SITE equip=$EQUIP point=$POINT"
```

> `display_name` is **required** on `CreatePoint` (and on sites/equips) — omitting it
> returns `422 missing field display_name`. Point **create and GET** both wrap the
> point as `{keyexpr, point:{…}}` (only `keyexpr` is top-level), so the point id is
> `.point.id` — sites/equips return a bare object (`.id`).

✅ All three ids are non-empty UUIDs. `GET /api/v1/points/$POINT | jq .keyexpr`
returns `nube/hq/ahu-3/temp`. Field accessors below use `.point.*`.

---

## 5. Confirm the live `cur` value is flowing

The sim publishes a new sample every `period_secs` (2s). The server's bus
subscriber updates `points.cur_value` and appends history on each sample.

```bash
sleep 5
curl -s $BASE/api/v1/points/$POINT | jq '{cur_value:.point.cur_value, cur_ts:.point.cur_ts}'
# → cur_value oscillating in [19.0, 23.0]; cur_ts recent
```

✅ `cur_value` is non-null and within `baseline ± amplitude` (19.0–23.0). Re-run
after a few seconds — it changes (deterministic triangle wave, not random).

---

## 6. Confirm history is accumulating

```bash
curl -s "$BASE/api/v1/points/$POINT/his?limit=10" | jq 'length'   # → grows over time
curl -s -X POST $BASE/api/v1/query -H content-type:application/json \
  -d '{"sql":"SELECT count(*) AS n FROM his"}' | jq
# → {"rows":[{"n":<N>}], …}  N climbs as the sim publishes
```

✅ History `length` > 0 and grows on re-run; the `his` count is non-zero.

**If `cur_value` stays null but the sim log shows publishes:** the point keyexpr
doesn't match the published key, or the bus isn't subscribed. See
[../feedback-loop/TRIAGE.md](../feedback-loop/TRIAGE.md) → "Sim publishing but no
cur landing".

---

## You now have a live stack

Next: open a feature runbook in [../features/](../features/), or run an
end-to-end script in [../scenarios/](../scenarios/).

## Teardown

```bash
# Ctrl-C the server in its terminal — graceful shutdown stops dispatcher →
# scheduler → supervisor, reaping the sim driver so its liveliness token clears.
make kill                                   # free :8088 / :5180 if a run was left bound
# `make kill` frees ports but does NOT reap a backgrounded `cargo run` server or
# its spawned drivers — if you ran `make dev-be &` (not Ctrl-C), kill them too, or
# an orphan keeps the SQLite WAL open and the next boot sees stale data:
pkill -f 'target/debug/rubix$'; pkill -f rubix-driver-sim
rm -f rubix.db rubix.db-wal rubix.db-shm    # wipe the store for a clean baseline
```

> ⚠️ **Orphan gotcha:** the store's data lives in `rubix.db-wal` until checkpointed.
> If `rm` seems not to wipe (next boot shows old sites), an orphaned server still
> holds the file — `pkill` it first, then `rm`. Verify clean with
> `pgrep -af 'target/debug/rubix$'` (should print nothing).
