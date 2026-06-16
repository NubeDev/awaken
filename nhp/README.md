# NHP — power-metering management platform (POC)

NHP is a **power-metering management platform**: a thin domain + UI layer on the
already-built **rubix** backend. It manages the *configuration and presentation* of
metering infrastructure — tenants, sites, gateways, networks, meters, registers,
units, dashboards, and users — but it does **not** talk to hardware (no polling, no
Modbus). A separate polling service consumes the configuration NHP stores. The
entire NHP domain is **data** (collection records + tags) on rubix, not backend
code. See [docs/OVERVIEW.md](docs/OVERVIEW.md) for the full product picture and the
rubix split.

> **rubix is frozen.** NHP never edits rubix source — it is UI + data on the
> unchanged rubix binary. Gaps in rubix are handled the NHP way (a data/UI
> workaround) and logged for the rubix team; see "Known POC limitations" below.

## How to run

**Toolchain:** node/pnpm live under nvm (not on `PATH`); cargo under `~/.cargo/bin`.
The Makefile resolves these. Ports: **backend `8094`, UI `5194`** (chosen so NHP
runs alongside a default rubix on 8092/5192).

**Credentials** come from `ui/.env.example` (copy to `ui/.env` for local dev). They
are the principals rubix `--seed-dev` casts in namespace `acme`:

| Var | Default | Used by |
| --- | --- | --- |
| `VITE_RUBIX_SUBJECT` / `VITE_RUBIX_SECRET` | `acme_operator` / `operator-demo` | the records API (dashboards, meter-types, gateways, wizards) |
| `VITE_RUBIX_ADMIN_SUBJECT` / `VITE_RUBIX_ADMIN_SECRET` | `acme_admin` / `admin-demo` | the principals/admin API (Users screen — needs `Role::Admin`) |

### Run order (established by WS-03)

`--seed-dev` provisions the `acme`/`globex` namespaces and the operator/admin
principals NHP authenticates as — it is a **prerequisite** for the NHP seed, which
is a separate step that writes the portfolio over the HTTP records API.

```sh
# 1. boot rubix --seed-dev (namespaces + principals) + the UI
make -C nhp dev SEED=1
#    → backend on http://127.0.0.1:8094, UI on http://127.0.0.1:5194

# 2. in a second shell: register the 7 collection definitions + seed the portfolio
make -C nhp seed
#    (writes 2 tenants / 4 sites / gateways / 485+ethernet networks / 14 meters /
#     105 registers, tagged for dashboard auto-build, + faked status/last_seen and
#     ~4368 history rows for history=true registers)

# 3. confirm the expected counts
make -C nhp seed-check
```

Then open **http://127.0.0.1:5194**. The sign-in screen takes a rubix bearer
token; for the seeded demo you can leave it blank/continue — the UI falls back to
the `VITE_RUBIX_*` service-account credentials above when no token is set.

### One-command smoke (no UI, gates the POC)

`make smoke` runs the whole thing headlessly against a **throwaway** `--seed-dev`
backend (its data dir lives inside `rubix/` — gitignored — and is removed on exit):

```sh
make -C nhp smoke
# boot rubix --seed-dev → register collections → seed → seed-check →
# records-check (dashboard data pipeline returns rows) → pnpm build → pnpm test:unit
```

See [docs/DEMO.md](docs/DEMO.md) for the stakeholder click-path.

## Demo walkthrough (short)

Log in → **Wizards → Gateway + networks** (add a gateway with 30 networks) →
**Admin → Meter-types** (add a type + register map) → **Wizards → Bulk meters**
(stamp meters onto a network) → **Dashboards** (drill tenant → site → gateway →
meter; see the auto-built trend chart, status rollup, and alarm panel). Full step
list with expected results: [docs/DEMO.md](docs/DEMO.md).

## Known POC limitations (honest)

This is a POC on a **frozen** rubix. Several rubix capabilities the design assumes
are not yet in the binary; NHP takes a documented data/UI workaround for each and
logs the upstream ask in [docs/sessions/TODOs.md](docs/sessions/TODOs.md). Nothing
below blocks the demo.

- **rubix is frozen** — NHP adds zero rubix Rust. Every gap is a workaround + a
  `RUBIX-TEAM` TODO, never a core patch.
- **Enums enforced client-side.** rubix has no `Select`/enum `FieldType`; closed
  enums (`net_type` 485/ethernet, `protocol`, register datatype, chart type) are
  modelled as `text` and the allowed set is enforced by the NHP client + UI
  dropdowns, **not** the gate.
- **`unique` and the per-network device limit are enforced client-side.** The gate
  validates `required` + field *type* only — it checks neither `unique` nor any
  `writeRule` predicate. Uniqueness and `network.max_devices` are enforced in the
  NHP layer (`collections/enforce.mjs`, `capacity.ts`) and the wizards. Defence in
  one place until the gate enforces it.
- **No `POST /query/batch`.** Only `POST /query` (a single SQL statement) exists.
  Dashboards therefore read the seeded data over the plain `/records` API and do
  windowing/grouping/aggregation **client-side** (one fetcher:
  `ui/src/features/dashboards/query/batch.ts`). The pure auto-build + widget layers
  don't change if a real batch lands — only that file's fetch swaps.
- **No `/ws/records` live feed in the UI.** Dashboards refresh on a
  visibility-aware **timer** (paused when the tab is hidden), not a live
  subscription.
- **No `/prefs` endpoint, so units are raw labels.** The `rubix-prefs` converter
  exists but isn't wired to HTTP; per-register unit metadata is stored, but live
  unit conversion/formatting waits on the endpoint. Units display as their raw
  labels.
- **Tenants are records, not namespaces.** NHP models a tenant as a `kind:"tenant"`
  record (tagged `tenant:<key>`) inside the single seeded `acme` namespace — there
  is no HTTP path to provision a true rubix namespace per tenant. No namespace
  isolation between tenants in the POC.
- **Tags live in `content.tags`, not graph edges.** rubix exposes no HTTP route to
  attach a tag-graph edge, so NHP carries its standard tags in the record's
  `content.tags` array. Auto-build reads `content.tags`; it cannot use rubix's
  server-side tag-graph `?tag=` filter.
- **File/blob fields out of scope.** The `File` field type exists but the blob
  store isn't built; site floor-plans / gateway photos are not part of the POC.

## Layout

- `collections/` — the 7 NHP collection definitions (data registered over rubix's
  records API) + the client-side enum/unique enforcement.
- `seed/` — the mock portfolio + faked poller data (status/last_seen + history),
  the `seed-check`, and the dashboard `records-check`.
- `scripts/smoke.sh` — the end-to-end smoke entrypoint (`make smoke`).
- `ui/` — the React/Vite app (auth shell, dashboards, admin, wizards).
- `docs/` — product docs (OVERVIEW, DOMAIN-MODEL, ADMIN, WIZARDS, DASHBOARDS,
  SEED), the DEMO walkthrough, and the session log under `docs/sessions/`.
