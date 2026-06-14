# Feature — Zenoh Data Plane

> Verified: **verified** on `rubix-gaps` (2026-06-13). Every gate is proven by the
> `rubix-server` `api_tests::bus` suite, which drives a **real second zenoh peer**
> (subscribe + `get`) against a live server — the exact rig this runbook describes.
> 7/7 green. Source: `rubix-server/src/bus/`,
> `rubix-server/tests/api_tests/bus.rs`, `rubix-driver-sim/tests/supervised.rs`.

Covers: live `cur` pub/sub, the `**/write` and `**/his/**` queryables, owned-site
scoping (a node answers only for sites it holds), and spark publishing. This is the
edge↔mesh integration layer.

Prereq: stack up with `RUBIX_ZENOH=1` (default) and the sim driver per
[../00_setup/QUICKSTART.md](../00_setup/QUICKSTART.md). A second peer session (a
small `zenoh` client, or `zenohd` + the CLI) is useful to observe the mesh.

---

## What to prove

1. The sim's `cur` publishes reach the server and land in the store (`cur_value` +
   history).
2. The server publishes its own `cur` on write/relinquish/ingest.
3. `**/write` and `**/his/**` queryables answer **only for owned sites** and stay
   silent for keys the node doesn't hold.
4. Sparks publish on `{org}/{site}/spark/{rule}/{id}`.

---

## Runbook

### 1. Inbound cur (driver → server → store)

Provision `nube/hq/ahu-3/temp` (QUICKSTART step 4) so the sim's publishes resolve.

```bash
sleep 5
curl -s $BASE/api/v1/points/$POINT | jq '{cur_value, cur_ts}'
```

✅ `cur_value` oscillates in `[19.0, 23.0]`; `cur_ts` is recent — the sim's zenoh
`put` on `nube/hq/ahu-3/temp/cur` was subscribed, resolved to the point, and stored.

### 2. Outbound cur (server publishes on write)

Subscribe a second peer to `nube/hq/ahu-3/**/cur`, then:

```bash
post /api/v1/points/$POINT/cur '{"value":42.0}'
```

✅ The peer observes a `cur` sample carrying 42.0 on `nube/hq/ahu-3/temp/cur` — the
server republishes effective-value changes onto the mesh.

### 3. Write queryable — owned site answers

From a peer, issue a zenoh **query** (get) on `nube/hq/ahu-3/sp/write` with a
priority-array command payload.

✅ The node (which owns `nube/hq`) replies, the write applies through the priority
array, and the new effective value publishes back on `cur`.

### 4. Owned-site silence

Issue a `**/write` or `**/his/**` query for a site the node does **not** have a row
for (e.g. `other/site/...`).

✅ The node stays **silent** — no reply, no "not found" noise. This is by design
(`Capability::covers` over owned sites); a different node would answer for its own.

### 5. Sparks on the bus

Subscribe the peer to `nube/hq/spark/**`, then:

```bash
post /api/v1/sparks '{"site_id":"'$SITE'","rule":"test-rule","severity":"warning","message":"hi","point_ids":["'$POINT'"]}'
```

✅ The peer receives a publish on `nube/hq/spark/test-rule/{id}`. (This is exactly
what the agent dispatcher subscribes to — see [AI_TOOLS_AND_AGENT.md](AI_TOOLS_AND_AGENT.md).)

---

## Acceptance criteria ("done")

- [x] Driver `cur` lands in the store (`cur_value` + history) — `driver_cur_publication_lands_in_the_store`.
- [x] Server republishes `cur` on write/relinquish/ingest — `cur_ingest_publishes_on_zenoh`.
- [x] `**/write` queryable applies a command for an owned site — `write_queryable_commands_priority_array`.
- [x] `**/his/**` queryable returns history for an owned site — `his_queryable_serves_history`.
- [x] A query for an unowned site gets silence (not an error) — `write_query_for_unowned_site_gets_no_reply`.
- [x] Sparks publish on `{org}/{site}/spark/{rule}/{id}` — `spark_create_publishes_on_zenoh`, `board_emit_spark_publishes_on_zenoh`.

---

## Gotchas

- **`RUBIX_ZENOH=0` disables this whole layer** *and* the supervisor (no sim
  driver) *and* subscription-triggered boards. The HTTP API still works over the
  store; you just have no live data.
- **Single-node peer mode needs no router.** Multi-node tests can point peers at a
  `zenohd` router, but it isn't required for the QUICKSTART path.
- Owned-site silence is the most surprising behavior — see TRIAGE §7. It's correct,
  not a bug.
- Live coverage already exists for the spawn→attach→publish→shutdown lifecycle in
  `rubix-driver-sim/tests/supervised.rs`; this runbook proves the server side.

## Known issues / fixes

### 2026-06-13 — verified end to end against a real second peer
All six acceptance gates green via `rubix-server` `cargo test --test api bus::` (7
tests, one per gate plus the write-queryable `cur` echo), which opens a genuine
second `zenoh::Session`, subscribes to `cur`, and issues `get`s on `**/write` /
`**/his/**` against a live server — the runbook's "second peer to observe the mesh".
Backend correct on every gate, including the surprising **owned-site silence** (a
`**/write` get for an unowned `ghost/x` site delivers *no* reply, not a "not found").
Manual single-node curl checks (Gates 1 & 5) also reconfirmed live. No backend or doc
bug found this round; the only historical bug is the inbound-subscriber fix below.

### 2026-06-13 — inbound driver `cur` never landed in the store
- **Symptom:** sim driver published `cur` (log: liveliness token declared,
  attached to bus) and the point's keyexpr matched, but `cur_value` stayed `null`
  and `his` stayed empty (QUICKSTART step 5/6, scenario S1).
- **Evidence:** testing/.evidence/phase0-cur-ingest/ (before + after bundles).
- **Root cause:** `crates/rubix-server/src/bus/` had `publish_cur` and the
  `**/write` + `**/his/**` queryables but **no inbound `**/cur` subscriber** — the
  driver→store half of the data plane was unimplemented (STATUS.md described it as
  done; it wasn't).
- **Fix:** added `crates/rubix-server/src/bus/subscribe_cur.rs` — a `**/cur`
  subscriber declared with `allowed_origin(Locality::Remote)` so it ingests only
  *driver* (other-session) publications, never the server's own `publish_cur`
  echoes (no feedback loop). It strips `/cur`, resolves the keyexpr via
  `point_by_keyexpr`, and calls `store.ingest_cur`. Wired into `serve()`.
- **Verified:** clean-DB sim run lands `cur_value` (oscillating 19–23) + history;
  `/query` count == direct sqlite count; an HTTP cur ingest on an isolated sensor
  produces exactly +1 his row (no self-ingest). Regression test
  `api_tests::bus::driver_cur_publication_lands_in_the_store`. `cargo test
  -p rubix-server` green (82 api + 65 unit); clippy clean.
