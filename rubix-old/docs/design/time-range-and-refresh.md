# Global Time Range & Auto-Refresh

Scope for a **dashboard-level time-range picker + refresh control** whose
`{from, to, interval}` flows into every time-aware widget query, with shareable URL
state, auto-refresh, and zoom-by-drag on history charts. This converts the dashboard
from a static grid of fixed queries into a live, interactive surface — the single
biggest user-visible gap for a monitoring product.

Mirrors nexus WS-01, re-grounded in rubix. It builds on
[variables-and-templating.md](variables-and-templating.md) (the interpolation engine
that substitutes the resolved range into SQL) and supplies the `$__from`/`$__to`/
`$__interval` built-ins that doc references.

## Problem

Rubix has **time bounds in exactly one place and nowhere else**:

- History reads accept a range: `GET /api/v1/points/{id}/his?start=&end=&limit=`
  (`HisQuery { start, end, limit }`, `crates/rubix-server/src/api/his/query.rs`).
  This is per-point, server-side, and not driven by any dashboard control.
- The dashboard query paths have **no time concept**: `POST /api/v1/query` is
  `{ sql }` (`api/query/run.rs`); `POST /api/v1/datasources/{id}/query` is
  `{ sql, params }` (`api/datasources/run.rs`). Neither carries a range.
- The UI has **no time picker and no auto-refresh control** (verified: no
  `timeRange`/`now-`/`auto-refresh` in `ui/src`). The only liveness is a blanket
  `LIVE_INTERVAL` 5 s poll baked into hooks (`ui/src/api/hooks.ts`), not user-tunable
  and not range-aware. History widgets fetch *all* history and slice client-side
  (`point_history` rendering, `ui/src/features/builder/components/widget-card.tsx`).

Consequence: a user cannot say "show me the last 6 hours" or "the last 7 days" and
have the whole board respond; cannot pause/resume refresh; cannot share a board at a
fixed point in time.

## Scope

### 1. Time-range model + store

`ui/src/stores/time-store.ts` (new zustand store, alongside the existing
`auth-store.ts`): `{ from, to }` where each is an absolute ISO instant **or** a
relative token (`now`, `now-6h`, `now/d`). A resolver turns the relative range into
concrete `{ fromTs, toTs }` at query time. Plus `{ refresh }` (off | 5s | 10s | 30s |
1m | 5m | 15m).

### 2. TimeRangePicker UI

`ui/src/features/time/**` (new): quick ranges (Last 5m / 15m / 1h / 6h / 24h / 7d /
30d, Today, Yesterday), absolute from/to with a calendar, a relative input, plus the
refresh-interval dropdown and a manual refresh button. Mount in the builder toolbar
(`ui/src/features/builder/index.tsx`) / `page-header.tsx`.

### 3. Auto-refresh loop

When an interval is set, invalidate the dashboard's widget data queries on a timer
(bump a `tick` floor folded into the query key, or TanStack `refetchInterval`).
**Pause when the tab is hidden.** This replaces the hard-coded 5 s `LIVE_INTERVAL`
for dashboard widgets with a user-controlled interval (keep the 5 s default as the
"live" preset so current behaviour is preserved when nothing is chosen).

### 4. Wire `{from, to, interval}` into queries

Extend the query DTOs (`QueryRequest`, `DatasourceQueryRequest`) with
`time_range: Option<{from, to}>` and `interval_secs: Option<u32>`, and feed them to
the [variables interpolation engine](variables-and-templating.md) so time macros in
widget SQL substitute:

- `$__from` / `$__to` → the resolved range bounds (bound parameters).
- `$__timeFilter(col)` → `col >= $from AND col < $to`.
- `$__timeGroup(col, '$__interval')` → bucketing by the resolved interval.

Widgets whose SQL uses no time macro are unaffected (back-compat). For `point_history`
widgets, thread the resolved range into the existing `his` `start`/`end` params rather
than fetching-all-and-slicing.

### 5. URL state

Reflect `?from=&to=&refresh=` in the URL; restore on load; shareable. (Rubix's first
URL-param state lands in the variables doc; share that mechanism.)

### 6. Zoom-by-drag

Dragging a range on a history/line chart sets the global time range, with a
"zoom out / back" affordance. Charts already render in
`ui/src/features/builder/components/widget-card.tsx` — hook the chart library's
brush/zoom event to the time store.

### 7. Per-widget time override (stretch)

A widget may shift or opt out of the global range (`timeShift`, relative override).
Low priority; spec'd here so the model leaves room for it.

## Design notes

- **`$__interval` auto-calculation:** derive a bucket from `(to - from) /
  targetDataPoints` (widget pixel width is a fine proxy) so `$__timeGroup` yields ~N
  points. Compute client-side and pass `interval_secs`, or in the engine — decide
  during build with the variables engine owner.
- **Freeze one `now` per refresh.** Resolve `now` once per dashboard refresh and pass
  the resolved instant to every widget so a fan-out of widgets shares a single
  instant (no per-widget clock skew). Let the server resolve `now` authoritatively to
  avoid client/server skew.
- **Cache-key snapping.** Fold the resolved `{fromTs, toTs}` (snapped to the refresh
  tick, not raw milliseconds — raw ms busts cache every render) and `interval_secs`
  into the widget data query key, so the range participates in invalidation cleanly.

## What to prove

1. The picker is in the toolbar; selecting "Last 6h" re-runs every widget whose SQL
   uses a time macro.
2. Auto-refresh runs at the chosen interval, pauses on a hidden tab, and manual
   refresh works.
3. `from/to/refresh` survive a reload and are shareable via URL.
4. A widget with `$__timeFilter(ts)` returns only in-range rows; one without is
   unchanged.
5. Drag-zoom on a history chart updates the global range; "zoom out" restores.
6. All widgets in one refresh share a single frozen `now`.

## Acceptance criteria

- [ ] Time store + relative-range resolver (`now-6h`, `now/d` rounding).
- [ ] TimeRangePicker + refresh-interval control mounted in the builder toolbar.
- [ ] Auto-refresh loop honouring the interval, paused on hidden tab; manual refresh.
- [ ] `QueryRequest`/`DatasourceQueryRequest` carry `time_range` + `interval_secs`;
      `$__from`/`$__to`/`$__timeFilter`/`$__timeGroup`/`$__interval` substitute via
      the variables engine (bound, injection-safe).
- [ ] `point_history` widgets use the resolved range against `his` `start`/`end`.
- [ ] `?from/to/refresh` URL round-trip.
- [ ] Drag-zoom updates and restores the range.
- [ ] One frozen `now` per refresh across all widgets; cache key snapped to the tick.
- [ ] Tests: range resolver (relative→absolute, `now/d`), cache-key snapping, macro
      substitution integration, picker logic.

## Out of scope (hand off)

- The interpolation engine itself → [variables-and-templating.md](variables-and-templating.md)
  (this consumes it).
- Generic variable interpolation → same doc.
- Annotations / event overlays on the time axis → later.
