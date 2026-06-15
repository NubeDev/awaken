# NHP — Dashboards

Dashboard **pages are auto-built** from the tag graph as a customer adds tenants,
sites, gateways, and meters — no manual board authoring for the common case. The
rendering and query machinery is rubix's; NHP supplies the *auto-build rules* and
the *tag conventions* that drive them. The authoritative chart/query design is
[DASHBOARDS-SCOPE.md](../../rubix/docs/design/DASHBOARDS-SCOPE.md); read it for
FieldConfig, units, batch query, bucketing, and the recharts/ECharts plan. This doc
is NHP-specific: how pages get built and what they show.

## Auto-build: tags → pages

When the wizards (see [WIZARDS.md](./WIZARDS.md)) create records, they apply the
standard tags from [DOMAIN-MODEL.md](./DOMAIN-MODEL.md). The auto-builder walks the
hierarchy and emits a page per level:

| Page | Built from | Shows |
| --- | --- | --- |
| **Tenant** | `tenant:<key>` | site cards: online/offline rollup, alarm count, total meters |
| **Site** | `site:<key>` | gateway cards + a site-wide summary (e.g. total power across meters via `quantity:power`) |
| **Gateway** | `gateway:<key>` | network list with device counts vs `max_devices`, gateway online/offline + `last_seen` |
| **Meter** | `meter:<key>` | one chart **per `chart_group`** (all voltages together, all currents together…), stat tiles for single registers, alarm panel, online/offline + `last_seen` |

The builder is deterministic: same tags ⇒ same page. A board is still a rubix
`kind:"board"` record with panels — the auto-builder *generates* one; an admin can
also save a hand-tuned board that overrides the generated one for a given scope.

## Chart grouping (tags)

Grouping is the tag mechanism from [ADMIN.md](./ADMIN.md):

- Registers sharing a `chart_group` are tagged `group:<group>`. The meter page
  renders **one multi-series chart per group** — e.g. `group:voltage` →
  V L1 / L2 / L3 on one line chart.
- Cross-meter grouping uses `quantity:<q>` — e.g. a site page can show total or
  per-meter `quantity:power` in one panel.
- Because grouping is authored at the **meter-type** level, every stamped meter
  inherits the same groupings automatically.

## Online / offline & stats

- `gateway.status`, `meter.status`, and `last_seen` are **written by the polling
  service** and read by NHP (NHP never polls — see
  [DOMAIN-MODEL.md](./DOMAIN-MODEL.md)). Dashboards render them as status tiles and
  roll them up the hierarchy (a site is "degraded" if any gateway is offline).
- Realtime via rubix **`/ws/records`** live queries (row-filtered per principal):
  status flips and new values push to the open board without a manual refresh.
- "Awaiting first poll" state for freshly-added, never-yet-seen devices.

## Alarms / thresholds

- Defined as register `alarm` thresholds (see [DOMAIN-MODEL.md](./DOMAIN-MODEL.md)
  §Alarms) — the **same threshold ramp** that colours the chart.
- A rubix **rule** evaluates thresholds and writes alarm/insight records; the
  dashboard subscribes to those over live queries and shows an alarm panel +
  per-chart colouring + hierarchy rollup (alarm count on tenant/site cards).
- Operators can **acknowledge** alarms (a gate-audited write); viewers cannot.

## Query / refresh / units

These come from rubix and are already built or planned in
[DASHBOARDS-SCOPE.md](../../rubix/docs/design/DASHBOARDS-SCOPE.md):

- **One batched query per board** via `POST /query/batch` (≤50 panels) — built.
- **Visibility-aware auto-refresh** (`Off · 5s · … · 5m`), snapped cache keys,
  `keepPreviousData` — DASHBOARDS-SCOPE §6.
- **UTC windows + epoch-aligned bucketing + relative tokens** (`now-1h`) —
  DASHBOARDS-SCOPE §5. Charts show **site-local** time using the site `timezone`.
- **Units**: register `unit`/`quantity` feed the `rubix-prefs` registry; live
  conversion/formatting waits on the `/prefs` endpoint (OVERVIEW gap #3) — until
  then, fixed unit labels.
- **History toggle**: only `history = true` registers offer a trend chart; others
  render as live stat/gauge tiles.

## UI base

The dashboard UI is ported from `rubix-old/ui` (see [SEED.md](./SEED.md) and the
kickoff). It already carries `features/` for builder, points, time/refresh, tenants,
admin, and settings — the NHP work is wiring the auto-build rules and NHP entities
onto that base, and porting the `nexus` widget set to recharts per
DASHBOARDS-SCOPE §8 (line/bar/area/pie easy; gauge/heatmap on a lazy ECharts
island).
