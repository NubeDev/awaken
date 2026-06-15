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
| WS-01 | App skeleton + Makefile (UI ported from rubix-old/ui, wired to rubix backend) | ⬜ | | | |
| WS-02 | NHP collection definitions (tenant→…→register) + enum strategy | ⬜ | | | |
| WS-03 | Seed: mock portfolio + faked poller data (status/last_seen/history) | ⬜ | | | |
| WS-04 | Admin: meter-types & register-map editor (history/unit/chart/group/alarm) | ⬜ | | | |
| WS-05 | Admin: gateways, networks (485/ethernet), device-limit, users/teams/roles | ⬜ | | | |
| WS-06 | Onboarding wizards (tenant→site→gateway+N networks→meters→users) | ⬜ | | | |
| WS-07 | Dashboards: tag-driven auto-build pages + online/offline + alarms | ⬜ | | | |
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
