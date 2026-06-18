# NHP — Seed & Makefile

NHP needs a one-command dev experience: build + run + populate a realistic mock
portfolio. Both are **copied from rubix** and adapted to the NHP domain.

## Makefile (copy from rubix)

Copy [rubix/Makefile](../../rubix/Makefile) as the NHP base. It already provides:

| Target | Does |
| --- | --- |
| `make build` | backend + UI |
| `make dev` | run backend + UI together (Ctrl-C stops both) |
| `make dev SEED=1` | …and populate the demo portfolio |
| `make dev-be` / `make dev-ui` | run one side |
| `make test` / `make lint` / `make fmt` | backend + UI |
| `make db-up` / `make db-down` / `make db-clean` | local TimescaleDB (Postgres) container |
| `make kill` | free dev ports |

Adapt: ports (rubix uses `BE_PORT ?= 8092`, `UI_PORT ?= 5192` — pick NHP values to
avoid clashing if both run), `UI_DIR`, and the `SEED` flag wiring (below). The
`FEATURES=cloud` switch (Postgres store + multi-tenant) carries over unchanged.

> Toolchain note: node/pnpm live under the nvm path, not on `PATH`; cargo is under
> `~/.cargo/bin`. The Makefile already resolves these — keep that resolution when
> copying.

## Seed (model after rubix's `seed_dev`)

Rubix seeds via a `--seed-dev` arg ([main.rs](../../rubix/crates/rubix-server/src/main.rs))
that builds a demo portfolio through the gate
([seed/portfolio.rs](../../rubix/crates/rubix-server/src/seed/portfolio.rs)) —
**every node is a normal record written through the gate**, so the seeded store has
real audit rows, undo history, and live-query events. NHP follows the same pattern,
swapping the Haystack-ish `site→equip→point→reading` shape for the NHP domain.

### What the NHP seed creates

A complete mock portfolio exercising the full model (see
[DOMAIN-MODEL.md](./DOMAIN-MODEL.md)):

- **Collections** first: register the NHP collection definitions (tenant, site,
  gateway, network, meter, meter-type, register) — the meta-collection bootstrap
  rubix already runs makes this a normal write.
- **1–2 meter-types** with full Modbus register maps (e.g. a 3-phase power meter:
  voltage L1/L2/L3, current L1/L2/L3, kW, kWh, frequency, power factor) — each
  register with real `address`/`datatype`/`scale`/`unit`/`quantity`/`history`/
  `chart_type`/`chart_group`/alarm metadata.
- **2 tenants**, each with **2 sites**.
- Per site: **1–2 gateways**, each with a mix of **485 and ethernet networks**
  (with `max_devices` caps), and several **meters** stamped from the meter-types.
- **Extra device families** beyond power meters (seed/device-types.mjs) — modelled
  as meter-types too, stamped through the same meter/register pipeline:
  - **LoRa sensors** matched to a gateway via a **`lora` network** (net_type +
    protocol `lora`, region/spreading-factor params). Each carries a **`battery`**
    register with a **low-battery alarm** (`direction:'below'` ramp: warn ≤30%,
    critical ≤15%). Sub-types: pulse input (water → m³ / electrical → kWh),
    temperature, CO₂, and CO sensors (CO/CO₂/temp carry high-threshold ramps).
  - **Modbus IO** on a 485/ethernet bus: a **pulse input** (read a register, scale
    to energy — same shape as a power meter), an **on/off coil** (read state +
    write command), and a **read/write holding register**.
  Seeded use cases (Acme): a **switch-room over-temperature** alarm (LoRa temp,
  biased to trip), an **electrical pulse meter** (Modbus pulse), **carpark + toilet
  exhaust fans** (Modbus coils), and **carpark CO sensors** (one biased to high CO,
  one to a low battery) — so the alarm panel shows the new alarm types firing.
- **Tags** applied per [DOMAIN-MODEL.md](./DOMAIN-MODEL.md) so dashboards auto-build.
- **Users/teams**: an admin + an operator + a viewer per tenant, and a
  **polling-service service-account** principal with write grants.
- **Mock live data**: since NHP doesn't poll, the seed plays the poller's role —
  it writes plausible `status`/`last_seen` and a back-fill of **history** for
  `history = true` registers (reuse rubix's
  [seed/history.rs](../../rubix/crates/rubix-server/src/seed/history.rs) series
  generator), plus a ticker for live values during `make dev SEED=1`. This is the
  only place NHP fabricates device data, and it stands in for the external service.

### Seed conventions to keep

- Write **through the gate** as the tenant operator (real audit/undo/events) — do
  not bypass with direct store writes.
- Idempotent-ish: rubix's seed repopulates on every boot; keep that, and document
  that deleting the DB only clears until the next seeded run.
- Keep the seed **off by default** (`SEED=1` to opt in) so a fresh DB starts empty.
