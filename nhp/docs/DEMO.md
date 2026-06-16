# NHP — demo walkthrough

The stakeholder click-path: log in, onboard a gateway with 30 networks, define a
meter-type, stamp meters onto it, then watch the dashboard auto-build from the tags
and show history + a status rollup. Every screen here is a real route built in
WS-04..07.

## 0. Set up (one time)

Run the backend + UI and seed the portfolio (full detail in
[../README.md](../README.md)):

```sh
make -C nhp dev SEED=1     # backend :8094, UI :5194
make -C nhp seed           # second shell: collections + portfolio
make -C nhp seed-check      # confirm counts
```

Open **http://127.0.0.1:5194**.

## 1. Log in

- The sign-in screen asks for a rubix API token. For the seeded demo, **continue
  with the field blank** — the UI falls back to the `acme_operator` service-account
  credentials from `ui/.env` for the records API (and `acme_admin` for the Users
  screen).
- **Expect:** you land on **Dashboards** (`/dashboards`). The left nav shows
  *Operate → Dashboards* and *Configure → Admin / Wizards*.

## 2. Onboard a gateway with 30 networks (Wizards)

- Go to **Wizards** (`/wizards`) → **Gateway + networks**.
- Pick a site, name the gateway, choose how many networks to add — set **30** — and
  pick the link type per network (485 / ethernet) and `max_devices`.
- Review the preview (1 gateway + 30 network rows) and **Run**.
- **Expect:** a batch write of 31 records (gateway + 30 networks), each network
  tagged with its gateway/site so dashboards pick it up. The wizard reports
  per-record success. (WS-06 verified this at N=30.)

## 3. Define a meter-type + register map (Admin)

- Go to **Admin → Meter-types** (`/admin/meter-types`).
- **Create** a meter-type (or **clone** the seeded `acme-pm5560`). Add register
  rows — Modbus address, datatype, scale, **unit**, quantity, **history y/n**,
  **chart type**, **chart group**, and **alarm thresholds**. Bulk-import from
  CSV/JSON is also available.
- Save. Note the **version** rollup — bumping the type lets you re-apply the diff to
  existing meters (WS-04).
- **Expect:** the type is saved as a `kind:"meter-type"` record with its register
  array; enum fields (datatype, chart type) are constrained by the dropdowns
  (client-side enforcement — see README limitations).

## 4. Stamp meters onto a network (Wizards)

- Go to **Wizards** (`/wizards`) → **Bulk meters**.
- Pick the network from step 2, choose the meter-type from step 3, set a count and
  the starting Modbus unit id.
- **Expect:** the wizard blocks you if the count would exceed the network's
  `max_devices` (the capacity guard, `capacity.ts`). Within cap, it stamps N meter
  records, each carrying `meter_type_version`, the type's registers, and the
  hierarchy tags (`site:…`, `gateway:…`, `meter-type:…`) that drive auto-build.

## 5. Open the auto-built dashboard (Dashboards)

- Go to **Dashboards** (`/dashboards`). Pick a **tenant** (e.g. *acme*).
- Drill the breadcrumb: **tenant → site → gateway → meter**. Each level is a page
  *generated* from the records carrying that scope's tag — nothing is hand-authored.
- **Tenant / site page — status rollup:** site cards roll up gateway status (a site
  shows **degraded** if any gateway is offline) plus meter and alarm counts. The
  seed marks at least one gateway offline so the rollup has something to show.
- **Gateway page:** lists the gateway's networks against `max_devices` (the capacity
  badge).
- **Meter page — history:** one multi-series **trend chart per chart group** (e.g.
  all voltage registers in one line chart), **stat tiles** for the latest values,
  and an **alarm panel**. Threshold lines colour the chart via the same severity
  logic that flags alarms.
- **Controls:** a **time window** selector (default *now-24h*) and a
  **refresh** interval (visibility-aware timer — paused when the tab is hidden; the
  POC does not use a live `/ws/records` feed).
- **Expect:** real seeded values (~4368 history rows back-fill the trend). With the
  default seed, values stay under thresholds so the alarm panel is correctly empty;
  the crossing logic is unit-tested and colours the chart when a value crosses.

## 6. Users / roles (Admin)

- Go to **Admin → Users** (`/admin/users`). This talks to rubix's real
  `/principals` admin surface as `acme_admin` (the operator credential is not an
  admin, so this screen uses the separate admin credential).
- **Expect:** list principals, change roles, create a user (with a minted secret),
  and see service accounts (e.g. the polling-service principal). A non-admin
  credential gets a 403 here.

---

That's the loop: **describe** the metering world (wizards + meter-types) and rubix
stores it as tagged records; the **dashboards auto-build** from those tags and show
the faked poller's status + history. A separate polling service (not part of NHP)
would read this same configuration and do the actual hardware talking.
