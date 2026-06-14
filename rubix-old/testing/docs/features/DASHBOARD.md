# Feature — Site Dashboard

> Scope: **UI feature**, `rubix-gaps`. The dashboard is a pure read surface — five
> widgets that pull live REST data for the active site and derive every figure
> client-side. No dashboard-specific backend; it composes the same `/equips`,
> `/points`, `/points/{id}/his`, and `/sparks` endpoints the rest of the app uses.
> Source: `rubix/ui/src/features/dashboard/`.

Covers: the active-site overview page — KPI strip, demand chart, load breakdown,
equipment roster, and recent sparks. Every value is computed from real API data;
there are no synthetic series or placeholder numbers.

Prereq: stack up, a site selected (`useActiveSite`), and some history (run the sim
a while). Open the app at `/` (or wherever `Dashboard` is routed).

---

## Layout

[index.tsx](../../../ui/src/features/dashboard/index.tsx) lays out one page bound to
the active site (`siteId` threads into every widget):

```
PageHeader  ── "<site> · live overview"
KpiRow                         ← 4 stat cards, full width
DemandChart (2 col) │ LoadBreakdown (1 col)
EquipmentHealth (2 col) │ RecentSparks (1 col)
```

When no site is selected `siteId` is `undefined`; each widget renders its own empty
state rather than the page erroring.

---

## Widgets & data sources

| Widget | Hook(s) | Endpoint | Derivation |
| --- | --- | --- | --- |
| [KpiRow](../../../ui/src/features/dashboard/components/kpi-row.tsx) | `usePoints`, `useSparks`, `usePointHistory` | `/points`, `/sparks`, `/points/{id}/his` | demand `cur_value`; energy = trapezoid-integrate 24h kW → MWh; comfort `cur_value`; open-spark tallies |
| [DemandChart](../../../ui/src/features/dashboard/components/demand-chart.tsx) | `usePoints`, `usePointHistory` | `/points`, `/points/{id}/his` | actual vs **centred rolling mean** of the same series; 24h/48h/7d range slices |
| [LoadBreakdown](../../../ui/src/features/dashboard/components/load-breakdown.tsx) | `usePoints` | `/points` | donut of `submeter`-tagged points' `cur_value` |
| [EquipmentHealth](../../../ui/src/features/dashboard/components/equipment-health.tsx) | `useEquips`, `usePoints`, `useSparks` | `/equips`, `/points`, `/sparks` | equip is "fault" if any of its points has an unacked `fault` spark |
| [RecentSparks](../../../ui/src/features/dashboard/components/recent-sparks.tsx) | `useSparks` | `/sparks` | 6 latest by `ts`, reuses `SparkRow` from the sparks feature |

The KPI/demand/load/spark hooks all carry `refetchInterval: LIVE_INTERVAL` (5 s), so
the page is live without manual refresh.

### Point selection
Widgets pick their point by **tag**, not by id (`hasTag`, `src/api/tags.ts`), so the
dashboard works against any tagged site:
- Demand: `elec|energy|power` **and** `meter|kw` (or `unit === 'kW'`); KPI prefers a
  `total`-slug demand point.
- Comfort: `comfort` tag. Load split: `submeter` tag with a numeric `cur_value`.

---

## What to prove

1. Each widget renders real figures for a tagged, populated site.
2. Derivations are honest: demand baseline is a rolling mean of the **same** series;
   energy MWh is integrated from history; deltas are last-vs-24h-earlier.
3. Empty states show (not errors) when a site lacks a demand point / submeters /
   equipment / sparks, or when no site is selected.
4. Live refresh: a written `cur` value or new spark reflects within ~5 s.
5. Equipment fault status tracks unacked `fault` sparks on the equip's points.

---

## Runbook

### 1. Populated site renders
Select a site with meter + submeter + comfort points and some history.

✅ KPI strip shows current demand, energy-today MWh, comfort index, and open-spark
counts; demand chart draws actual + dashed rolling-average with an above/below badge;
load donut splits by submeter; equipment roster lists tiles; recent sparks list fills.

### 2. Derivations are real, not synthetic
Read [kpi-row.tsx:25-35](../../../ui/src/features/dashboard/components/kpi-row.tsx#L25-L35)
(`energyTodayMWh`) and [demand-chart.tsx:42-58](../../../ui/src/features/dashboard/components/demand-chart.tsx#L42-L58)
(`toRows` baseline).

✅ Energy is `Σ value·Δhours / 1000` over the last 24 h of samples; the baseline is a
centred window mean (`±8` samples) of the actual series. No constant or random data.

### 3. Empty states
Select a site with no submeters / no demand point / no equipment.

✅ "No submeters on this site." / "No demand history for this site yet." /
"No equipment on this site yet." / "No open findings." — each widget degrades
independently; the page never throws on `undefined` site.

### 4. Live refresh (~5 s)
`POST /api/v1/points/{id}/cur` a new demand value, or let the rules engine raise a
spark.

✅ KPI demand and the open-spark count update within one `LIVE_INTERVAL` tick (5 s)
without a reload (`refetchInterval` on `usePoints`/`useSparks`/`usePointHistory`).

### 5. Equipment fault propagation
Raise an unacked `fault` spark on a point belonging to an equip.

✅ That equip's tile flips to "Fault active" (red), and the card subtitle
`<n>/<total> nominal` decrements. Acking the spark restores "Nominal".

---

## Acceptance criteria ("done")

- [ ] All five widgets render live figures for a populated, tagged site.
- [ ] Energy MWh and demand baseline are derived from real history (no synthetic data).
- [ ] Each widget shows an independent empty state; no-site does not error.
- [ ] Live values refresh within ~5 s of a `cur` write or new spark.
- [ ] Equipment fault status tracks unacked `fault` sparks per equip.

---

## Gotchas

- **Tag-driven, not id-driven.** A site whose points lack `meter`/`submeter`/`comfort`
  tags shows empty widgets even with data present — the figures are correct, the
  *selection* found nothing. Tag the points (see ENTITY_CRUD_AND_TENANCY / tags).
- **History cadence.** `dayDelta` needs ≥50 numeric samples and reads "24h earlier"
  as `length-49` (the 30-min serving cadence), so a thin history yields no delta
  (`—`) rather than a wrong one.
- **`cur_value` can be a string.** `/points` may serve `cur_value` as `"19.0"`;
  `formatValue` handles display, but numeric widgets (load donut, energy) guard with
  `typeof === 'number'` and skip non-numeric points.
- The mount-time "fresh in last hour" tally is a snapshot (`Date.now()` read once at
  mount), not a live ticker — by design, to keep render pure for the React compiler.
