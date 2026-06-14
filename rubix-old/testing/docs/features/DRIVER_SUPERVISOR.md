# Feature — Driver Supervisor & Capability Scoping

> Verified: **verified** live on `rubix-gaps` (`bdbd01f7`, 2026-06-13). Every gate
> green; backend correct on all of them, no doc or backend bug found. Source:
> `rubix-server/src/supervisor/`, `rubix-driver/src/manifest/`,
> `rubix-driver-sim/`. Live coverage: `rubix-driver-sim/tests/supervised.rs`.

Covers: manifest loading, spawn/liveliness/backoff, capability scoping
(`covers`/access direction), and fail-closed behavior. This is the edge runtime
that brings physical/simulated drivers onto the mesh under a security grant.

Prereq: a built `rubix-driver-sim` and a `drivers.json` per
[../00_setup/SIM_DRIVER.md](../00_setup/SIM_DRIVER.md). `RUBIX_ZENOH=1`.

---

## What to prove

1. Manifests load from `RUBIX_DRIVERS`: missing file → no drivers; malformed → fail
   closed; valid → spawn.
2. A spawned driver attaches (liveliness token) and publishes.
3. Capabilities confine the driver: `covers` is path-boundary, access gates
   direction, out-of-grant publishes are refused **locally** before the bus.
4. Graceful shutdown reaps drivers (tokens clear).

---

## Runbook

### 1. Missing manifest file → no drivers (valid)

```bash
RUBIX_DRIVERS=/no/such/file.json make dev-be &
```

✅ Server boots, log notes no drivers; the API is healthy. (Cloud nodes run no local
drivers — this is a normal config, not an error.)

### 2. Malformed manifest → fail closed

```bash
echo '{ not json }' > bad-drivers.json
RUBIX_DRIVERS=bad-drivers.json make dev-be
```

✅ Boot **fails** with a named parse error — a typo in driver config must not
silently disable supervision.

### 3. Valid manifest → spawn + attach + publish

Use the QUICKSTART `drivers.json` (sim granted `publish` on `nube/hq/ahu-3`,
configured to publish `nube/hq/ahu-3/temp`). Boot normally.

✅ `server.log` shows the sim spawned and **attaching** (liveliness token); within a
few `period_secs` the point's `cur_value` lands (see [ZENOH_BUS.md](ZENOH_BUS.md)).

### 4. Capability `covers` — path boundary

Grant prefix `nube/hq/ahu-3`. By `Capability::covers`
(`rubix-driver/src/manifest/capability.rs:65`):
- covers `nube/hq/ahu-3` (self) and `nube/hq/ahu-3/fan/cur` (descendant)
- does **not** cover `nube/hq/ahu-30/fan` (sibling with a string prefix)

✅ A point under the grant publishes; a point under a string-prefix sibling does not.

### 5. Access direction

Grant `access:"publish"`. The driver may publish `cur` and reply to queries under
the prefix, but a **subscribe** under it is refused.

✅ Set `access:"subscribe"` instead and the publish is refused; `access:"all"`
permits both. (`Access` enum, `capability.rs:11`.)

### 6. Out-of-grant publish refused locally (fail closed)

Set `config.point` to `nube/hq/ahu-9/temp` while the grant is only `nube/hq/ahu-3`.

✅ The sim **attaches** (liveliness isn't gated) but its `ScopedSession` refuses
every `put` **before the bus** with a named denial — **no `cur` ever appears on the
mesh**. This is exactly `out_of_grant_publish_is_refused_locally` in
`tests/supervised.rs`.

### 7. Graceful shutdown reaps the driver

Ctrl-C the server.

✅ Shutdown stops dispatcher → scheduler → supervisor in order; the sim receives
SIGINT, clears its liveliness token, and exits — a peer watching liveliness sees the
token clear.

---

## Acceptance criteria ("done")

- [x] Missing `RUBIX_DRIVERS` file → boots with no drivers.
- [x] Malformed manifest → boot fails closed.
- [x] Valid manifest → driver spawns, attaches, publishes.
- [x] `covers` admits self/descendants, rejects string-prefix siblings.
- [x] Access direction gates publish vs subscribe.
- [x] Out-of-grant publish refused locally; nothing reaches the mesh.
- [x] Shutdown reaps the driver; liveliness token clears.

---

## Gotchas

- **`RUBIX_ZENOH=0` ⇒ no supervisor ⇒ no drivers**, regardless of `RUBIX_DRIVERS`.
- A manifest with **no capabilities is rejected** by `validate()` — every driver
  must declare a grant.
- Capability `covers` is the **same** primitive used for bus owned-site scoping,
  query scope, and agent tool scope — a bug here has wide blast radius. Don't
  "loosen" it to make a test pass.
- Backoff on driver crash is jittered exponential; a flapping driver won't
  hot-loop. A driver that never attaches gets reaped (`await_attach`).

## Known issues / fixes

Verified live 2026-06-13 (`bdbd01f7`). All seven gates green; backend correct on
every one, no doc or backend bug found. Each gate maps to a passing test:

- Gates 1, 2 (manifest load: missing → no drivers, malformed → fail closed) —
  `rubix-server` lib `supervisor::manifests::tests::{missing_file_is_no_drivers,
  malformed_file_fails_closed}`.
- Gates 3, 6, 7 (spawn + attach + publish; out-of-grant publish refused locally
  with a named denial and nothing on the mesh; shutdown reaps + liveliness clears)
  — `rubix-driver-sim/tests/supervised.rs::{supervisor_spawns_sim_which_attaches_and_publishes,
  out_of_grant_publish_is_refused_locally}` (the denied sim's log line
  `driver \`sim-denied\` is not granted \`publish\`` confirms gate 6).
- Gates 4, 5 (`covers` path boundary rejecting `ahu-30` sibling; access direction
  gating publish vs subscribe) — `rubix-driver` lib
  `manifest::capability::tests::{covers_self_and_descendants_but_not_siblings,
  access_gates_direction}`.
- Gotcha (no-capabilities manifest rejected by `validate()`) —
  `manifest::tests::driver_with_no_capabilities_is_rejected`.
