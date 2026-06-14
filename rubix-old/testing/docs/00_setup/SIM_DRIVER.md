# Sim Driver — The Live Data Generator

> Verified: code-grounded on `rubix-gaps` tip, 2026-06-13.
> Source: `crates/rubix-driver-sim/` and `crates/rubix-driver/src/manifest/`.

`rubix-driver-sim` is a **real, capability-scoped driver**, not a test stub. The
supervisor spawns it from a `drivers.json` manifest; it opens its own zenoh peer
session, declares a liveliness token, self-authorizes against its granted
capabilities, and publishes a deterministic `cur` oscillation until SIGINT. It is
how this suite gets live data without external hardware.

---

## How it's launched

You never run it by hand. The server's supervisor reads `RUBIX_DRIVERS`
(`drivers.json`) at boot and spawns one child per manifest, injecting three env
vars the sim reads (`crates/rubix-driver-sim/src/config.rs`):

| Env var | Carries | Constant |
|---------|---------|----------|
| `RUBIX_DRIVER_NAME` | the manifest `identity.name` | `ENV_DRIVER_NAME` |
| `RUBIX_DRIVER_CAPS` | the JSON-encoded `CapabilitySet` | `ENV_DRIVER_CAPS` |
| `RUBIX_DRIVER_CONFIG` | the opaque `config` blob | `ENV_DRIVER_CONFIG` |

`SimConfig::from_env()` fails closed if `name` is missing or `caps` won't parse —
it aborts before touching the bus.

---

## The manifest (drivers.json entry)

```json
{
  "identity": {
    "name": "sim",
    "protocol": "sim",
    "version": "0.1.0",
    "launch": { "command": "/abs/path/target/debug/rubix-driver-sim", "args": [] }
  },
  "capabilities": { "grants": [ { "prefix": "nube/hq/ahu-3", "access": "publish" } ] },
  "config": { "point": "nube/hq/ahu-3/temp", "period_secs": 2, "baseline": 21.0, "amplitude": 2.0, "cur_capacity": 64 }
}
```

- **`capabilities.grants[].access`** is one of `publish` / `subscribe` / `all`
  (`crates/rubix-driver/src/manifest/capability.rs`). The sim only needs
  `publish`.
- **`capabilities.grants[].prefix`** must be a clean keyexpr prefix: non-empty, no
  leading/trailing slash, no wildcards (`validate()` rejects those).
- A manifest with **no capabilities is rejected** — the supervisor fails closed.

### config blob (`RUBIX_DRIVER_CONFIG`)

| Field | Meaning | Default |
|-------|---------|---------|
| `point` | keyexpr prefix to publish on (publishes on `{point}/cur`) — **required** | — |
| `period_secs` | seconds between samples | `5` |
| `baseline` | value the sensor oscillates around | `21.0` |
| `amplitude` | peak deviation from baseline | `2.0` |
| `cur_capacity` | outbound buffer bound (latest-wins on overflow) | `64` |

---

## What it publishes

- **Keyexpr:** `{point}/cur`. With `point = nube/hq/ahu-3/temp` it publishes on
  `nube/hq/ahu-3/temp/cur` — the canonical `{org}/{site}/{equip-path}/{point}/cur`
  shape the server's bus subscriber expects.
- **Value:** a JSON number. `sample(baseline, amplitude, step)` is a **deterministic
  triangle wave** (no RNG), rounded to 0.1, staying within `baseline ± amplitude`.
  With the defaults it sweeps `[19.0, 23.0]` on a 12-step cycle. Re-runs reproduce
  the same sequence — this is what makes the determinism scenario (S4) possible.

---

## Fail-closed scoping (the safety property)

Before its first publish, the sim calls `caps.authorize_publish(name, "{point}/cur")`
(`crates/rubix-driver-sim/src/simulate.rs`). If the configured `point` is **not
under a granted prefix**, it logs a named denial and exits **without publishing
anything** — nothing reaches the mesh. The `ScopedSession` wrapper
(`src/scoped.rs`) re-checks every `put`/`declare_subscriber`/`get` against the
grant, so an out-of-scope operation is refused *locally*, before the bus.

This is covered live by `crates/rubix-driver-sim/tests/supervised.rs`:
- `supervisor_spawns_sim_which_attaches_and_publishes` — full spawn → attach →
  publish (values 19.0–23.0) → shutdown clears the liveliness token.
- `out_of_grant_publish_is_refused_locally` — point outside the grant: the sim
  attaches but no `cur` ever appears on the mesh.

---

## Knobs for shaping test data

| Want | Set |
|------|-----|
| faster samples | `config.period_secs: 1` |
| a different range | `config.baseline` / `config.amplitude` |
| a second point | add a second manifest entry with its own `point` + grant |
| a write-path test | grant `access: "all"` and have the driver subscribe to `**/write` (see `ScopedSession::get`) |
| verify fail-closed | set `config.point` outside `capabilities.grants[].prefix` → no data appears |

> A point you publish for must also exist in the store (`POST /api/v1/sites` →
> `equips` → `points`) for its `cur_value`/history to be recorded — the bus
> subscriber resolves the keyexpr to a stored point. See
> [QUICKSTART.md](QUICKSTART.md) step 4.
