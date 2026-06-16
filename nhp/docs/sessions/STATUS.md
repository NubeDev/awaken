# NHP POC Build — Workstream Queue

The unattended build queue for the NHP power-metering POC. Driven by
[setup/_ORCHESTRATION.md](./setup/_ORCHESTRATION.md). Each row is a workstream (WS) with a spec doc
in this directory. Status legend: ⬜ pending · 🔵 in-progress · ✅ done · ⛔ blocked (see TODOs.md).

Branch: **`nhp-poc`**. Product scope: [../OVERVIEW.md](../OVERVIEW.md) + the feature docs beside it;
file-layout standard: [../../../rubix/docs/FILE-LAYOUT.md](../../../rubix/docs/FILE-LAYOUT.md).

Queue order is dependency order — earlier rows ship contracts later rows build on. NHP is a thin
layer on the **already-built** rubix backend (see OVERVIEW build-status); the backend is NOT rebuilt
here. WS-01 stands up the NHP app skeleton + Makefile; WS-02 lands the collection definitions every
later WS reads/writes; the seed (WS-03) gives every UI WS real data to render.

| # | Workstream | Status | Started | Finished | Commit |
| --- | --- | --- | --- | --- | --- |
| WS-01 | App skeleton + Makefile (UI ported from rubix-old/ui, wired to rubix backend) | ✅ | 2026-06-15T23:46:06Z | 2026-06-16T00:01:17Z | 759b1b34 |
| WS-02 | NHP collection definitions (tenant→…→register) + enum strategy | ✅ | 2026-06-16T00:05:00Z | 2026-06-16T00:29:20Z | 749a4b86 |
| WS-03 | Seed: mock portfolio + faked poller data (status/last_seen/history) | ✅ | 2026-06-16T00:37:54Z | 2026-06-16T11:05:00Z | 75d9ac61 |
| WS-04 | Admin: meter-types & register-map editor (history/unit/chart/group/alarm) | ✅ | 2026-06-16T00:58:07Z | 2026-06-16T01:17:31Z | 12cc1ee0 |
| WS-05 | Admin: gateways, networks (485/ethernet), device-limit, users/teams/roles | ✅ | 2026-06-16T01:20:12Z | 2026-06-16T01:38:09Z | 98653028 |
| WS-06 | Onboarding wizards (tenant→site→gateway+N networks→meters→users) | ✅ | 2026-06-16T01:40:16Z | 2026-06-16T12:06:00Z | e7ee7e40 |
| WS-07 | Dashboards: tag-driven auto-build pages + online/offline + alarms | ✅ | 2026-06-16T02:08:36Z | 2026-06-16T12:35:00Z | (this commit) |
| WS-08 | POC polish: end-to-end smoke, README, demo walkthrough | ⬜ | | | |

## Dependency notes
- **WS-01** is the root: copy `rubix-old/ui` → `nhp/ui`, strip to an app shell + auth + nav, point
  the API client at the rubix backend (RUBIX_BIND), copy `rubix/Makefile` → `nhp/Makefile` adapting
  ports/dirs (see [../SEED.md](../SEED.md)). Everything depends on it.
- **WS-02** lands the collection records (`kind:"collection"`) for tenant/site/gateway/network/meter/
  meter-type/register per [../DOMAIN-MODEL.md](../DOMAIN-MODEL.md), and resolves the enum strategy
  (text+writeRule now, or add a `Select` field type to rubix — OVERVIEW gap #1). Every CRUD/UI WS
  depends on these shapes existing + validating.
- **WS-03** (seed) needs WS-02's collections; it writes a mock portfolio through the gate and fakes
  the poller (status/last_seen + history for `history=true` registers). Every UI WS renders against it.
- **WS-04/05** (admin) need WS-02 (shapes) + WS-03 (data to show) + WS-01 (app shell). WS-04 owns
  meter-types/registers; WS-05 owns gateways/networks/users — they touch different collections, but
  run sequentially per the one-branch rule.
- **WS-06** (wizards) needs WS-04/05's create paths and the device-limit rule; it orchestrates them.
- **WS-07** (dashboards) needs WS-02's tags + WS-03's history; ports the chart-builder/widget set per
  [../DASHBOARDS.md](../DASHBOARDS.md) and [rubix DASHBOARDS-SCOPE](../../../rubix/docs/design/DASHBOARDS-SCOPE.md).
- **WS-08** is the POC wrap: a scripted end-to-end (seed → wizard → dashboard), a README, fix
  whatever the smoke test surfaces. Depends on everything.

## Loop log
<!-- The loop appends one line per wake here: <utc> <action> (spawned WS-xx / gated WS-xx ✅ / blocked WS-xx ⛔ / idle). -->
- 2026-06-15T23:46:06Z spawned WS-01
- 2026-06-16T00:01:17Z gated WS-01 ✅ (pnpm -C nhp/ui build green, commit 759b1b34)
- 2026-06-16T00:05:00Z spawned WS-02
- 2026-06-16T00:29:20Z gated WS-02 ✅ (7 collections register + check green on unmodified rubix, zero rubix diff, commit 749a4b86)
- 2026-06-16T00:37:54Z spawned WS-03
- 2026-06-16T11:05:00Z gated WS-03 ✅ (make seed + seed-check green on --seed-dev rubix: 2 tenants/4 sites/5 gw/7 net/14 meters/105 registers/4368 history rows, idempotent re-run, zero rubix diff, commit 75d9ac61)
- 2026-06-16T00:58:07Z spawned WS-04
- 2026-06-16T01:17:31Z gated WS-04 ✅ (pnpm -C nhp/ui build green + 21/21 unit tests; meter-type CRUD/clone, register-map+alarm editor, CSV/JSON bulk import, versioning rollup + per-meter re-apply diff — all on the rubix records API, live-verified on --seed-dev, zero rubix diff, commit 12cc1ee0)
- 2026-06-16T01:20:12Z spawned WS-05
- 2026-06-16T01:38:09Z gated WS-05 ✅ (pnpm -C nhp/ui build green + 21/21 unit tests; gateway CRUD with required-site picker + read-only poller status/last_seen, network CRUD with 485/ethernet param sub-forms + device-limit capacity badge/cap-floor guard (capacity.ts), users/roles CRUD on the REAL rubix /principals admin API as seeded acme_admin (per-request auth override) — live-verified on --seed-dev: operator→403, principal create/patch/delete + minted secret, gateway+network round-trip with site; zero rubix diff, commit 98653028)
- 2026-06-16T01:40:16Z spawned WS-06
- 2026-06-16T12:06:00Z gated WS-06 ✅ (pnpm -C nhp/ui build green + 44/44 unit tests; reusable resumable wizard shell (stepper + batch-write with parentRef late-binding), gateway+N-networks verified at N=30, bulk-meters with capacity.ts cap-block + register/tag stamping, tenant(as-record)/site/user wizards, combined add-everything tree — all orchestrate WS-04/05 records-API create paths and apply the shared tag module (enums/tags.ts, parity-guarded against seed/tags.mjs); live-verified on --seed-dev: 31/31 gateway+30-net write + 27/27 combined-tree write with full tag chains; zero rubix diff, no stray rubix-data)
- 2026-06-16T02:08:36Z spawned WS-07
- 2026-06-16T12:35:00Z gated WS-07 ✅ (pnpm -C nhp/ui build green + 55/55 unit tests; tag-driven auto-build tenant→site→gateway→meter pages from content.tags via enums/tags.ts — site cards roll up gateway status (degraded if any offline) + meter/alarm counts, meter page renders ONE multi-series trend per chart_group + stat tiles + alarm panel, gateway page lists networks vs max_devices; recharts-only widgets (line/bar/area/stat/table/status/alarm), NO ECharts island; threshold ramp colours charts (reference lines) via the same severityFor that flags alarms; POC simplifications — fell back to /records + client-side window/group (rubix has NO /query/batch, only /query) + a visibility-aware refresh timer (not /ws/records), both documented in WS-07.md; live-verified on --seed-dev+make seed: 2 tenants/6 sites/5 gw/14 meters/105 registers/4368 history, acme-plant rolls up degraded, a register joins 48 history points; seeded values stay under thresholds so no alarm fires live (panel correctly empty; crossing logic unit-tested); zero rubix diff, no stray rubix-data)
