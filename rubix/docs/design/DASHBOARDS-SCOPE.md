# SCOPE — Dashboards, charts & the query pipeline

Feature scope for the **query → chart → board** surface. The near-term base is the
Laminar port (see [LAMINAR-BORROW.md](./LAMINAR-BORROW.md)); the **target** is a
Grafana-grade dashboard, and the model to grow into is the one already proven in
the `nexus` reference project (`/home/user/code/rust/starter/nexus`). This is a
*feature* scope; the platform scope and its principles stay authoritative in
[SCOPE.md](../SCOPE.md) — nothing here overrides "commands go through the gate,
reads are SurrealDB-native," the two-tier persistence map, or the scale path.

It settles the open design questions raised against
[`ui/src/components/dashboards/`](../../ui/src/components/dashboards):

1. **The headline call: backend or frontend for calculations & transforms?**
   (asked twice — for chart scaling, and for the whole nexus transform pipeline).
2. **User preferences** — datetime display, unit display & conversion.
3. A **bulk** query API (run a whole board at once).
4. An **optimised** query API (the repeated-field / repeated-scan problem).
5. Date/time correctness (it's wrong today) and bucketing.
6. Polling / auto-refresh / live.
7. Grafana parity — how powerful, and what model gets us there.
8. Stage 2 — porting the `nexus` widget set to recharts.

Each is answered with a decision and the reasoning, so this doubles as the build
sheet.

---

## 0. Current state (verified)

**Rubix today (the Laminar MVP).** A board is a `kind:"board"` record of
`panels: BoardPanel[]` ([api/boards.ts](../../ui/src/api/boards.ts)); each panel
points at a `kind:"chart"` record holding `{ sql, config }`
([api/charts.ts](../../ui/src/api/charts.ts)). The chart config is **tiny** —
`{ type, x, y, breakdown, displayMode }` over four types (line/bar/hbar/table)
([chart-builder/types.ts](../../ui/src/components/chart-builder/types.ts)). Each
[ChartPanel](../../ui/src/components/dashboards/ChartPanel.tsx) runs its **own**
`runQuery(api, sql)`; axis domains are computed in-browser; **no polling**; time
is spliced into SQL as `{{…}}` client-side. `POST /query` takes `{ sql }`,
returns `{ rows }`, rebuilds the **entire** DataFusion context per request (no
cache — [aggregate/mod.rs](../../crates/rubix-query/src/aggregate/mod.rs)).

**`rubix-prefs` exists but is not wired.** `Preferences { units, datetime }`
([preferences.rs](../../crates/rubix-prefs/src/preferences.rs)) with a real
metric↔imperial converter (`Quantity::{Temperature,Length,Mass,Speed}`,
[quantity.rs](../../crates/rubix-prefs/src/units/quantity.rs)) and an
`apply_to(dto, prefs, fields)` that rewrites declared DTO fields at the response
layer ([apply.rs](../../crates/rubix-prefs/src/apply.rs)). **Gaps:** no timezone
field (strftime only, UTC), **no HTTP endpoint**, and the frontend consumes none
of it — charts hard-code `en-US` and do no unit conversion.

**`nexus` (the Grafana-grade target).** Reference only, different repo. A panel is
`{ type, config: { query, fields, fieldConfig, options, transforms, live } }`.
Ten widget types (line/area/bar/scatter/heatmap/pie/gauge/stat/status/table), a
**FieldConfig** model (defaults + `byName`/`byRegex` overrides, multi-step
threshold ramps, value mappings, a 30+ unit registry, legend/axis/log-scale), a
client-side **transform** pipeline (rename/calculated/filter/groupBy/reduce),
template **variables**, relative-time tokens, visibility-aware refresh, and SSE
live streaming. Charts are **ECharts** behind a pure option-builder + registry
pattern. Its query API is `{ sql, time_range, interval_secs, variables, kind,
params, sources, insight } → { columns, rows, stats }` — no batch, but a
**kind-mode** (a reverse-DNS query id the backend resolves to SQL) as a
structured alternative to raw SQL, and an **insight** hook (post-query Rhai).

---

## 1. The headline call — **compute on the backend, present on the frontend**

The question "should the calcs be backend?" was asked of two different things, and
collapsing them gives the wrong answer. Split every operation into **compute** (it
needs data the client doesn't have, or it must be identical across every client)
versus **present** (it's a property of *this* rendering surface). The rule:

> **Compute on the backend. Present on the frontend, driven by portable config.**

This directly serves your "easier for clients, e.g. if I make an app later" goal —
but not by "do everything server-side." A thin client stays thin when the backend
returns **display-ready numbers** (bucketed, aggregated, unit-converted) **and**
the chart record carries **declarative presentation config** the app reads instead
of reimplementing. The app then needs only: call the query API → read the
FieldConfig → map to its toolkit. No business logic ported.

| Operation | Side | Why |
| --- | --- | --- |
| Time bucketing / grouping | **Backend** | needs rows the client doesn't hold; must match rule buckets (§5) |
| Aggregation (sum/avg/count/**p90/p95/p99**) | **Backend** | same — never fetch raw rows to aggregate in JS |
| Downsampling (large series → ~N points) | **Backend** | bounds payload; a native app benefits identically |
| Unit conversion (metric↔imperial) | **Backend** (`rubix-prefs`) | one converter, every client identical (§2) |
| Transforms — aggregate (filter/groupBy/reduce) | **Backend** | shrink payload; need the full dataset |
| Transforms — cosmetic (rename/calculated/organize) | **Frontend** (nexus executor) | trivial & instant; the *spec* is the portable contract |
| Axis domain / min-max / tick density | **Frontend** | pure function of the rows in hand; a round trip buys nothing |
| Threshold colours, value mappings, legend | **Frontend** (from config) | per-pixel render concern; config is portable, the painting isn't |
| Number/datetime *formatting* | **Frontend** (from prefs pattern) | needs the raw instant for tick math; format with a backend-supplied pattern (§2) |

**Transforms — hybrid, spec-as-contract (decided).** The portability lever is the
**declarative spec stored on the chart record**, *not* where it runs — so a future
client is served either way by reading the spec. Given that, split execution by
class: **payload-shrinking ops (filter/groupBy/reduce) run server-side** (SQL where
possible, a small DataFusion stage otherwise — they need the full dataset and
shrink the wire); **cosmetic ops (rename/calculated/organize) run client-side** via
`nexus`'s existing executor — they're trivial, deterministic, and keeping them
client-side makes builder edits instant instead of a round trip per tweak. Borrow
`nexus`'s transform *types* verbatim and **keep its executor** for the cosmetic
tier. **Promotion rule:** move a cosmetic op server-side only when data volume or a
genuine second client forces it — execution location is a per-op choice that can
change; the spec is the durable contract. This avoids the §4b "structured query
builder" iceberg (the spec is a post-query transform list, not a SQL compiler) and
avoids rebuilding working code as a maybe-needed DataFusion stage.

**What stays frontend, unavoidably:** axis scaling, tick selection, drag-zoom,
cross-chart sync, threshold/​mapping *painting*, legend layout. Even a native app
does these in its own toolkit — so we ship them as **portable config**, never as
server round-trips.

---

## 2. User preferences — **units convert on the backend, datetime formats on the frontend**

`rubix-prefs` is built; this wires it. Two prefs, two different paths, because a
chart needs raw instants for its time axis but only needs final numbers for value
axes.

**Units → backend conversion (use `apply_to`).** A chart's FieldConfig (§7)
declares each numeric column's `Quantity` (e.g. column `temp` is
`Quantity::Temperature`). The query/batch endpoint applies the **requesting
principal's** unit system per column via the existing converter, returning
display-ready numbers plus the unit label. Axis math then works on converted
values; a future app gets identical numbers for free. This is exactly what
[apply.rs](../../crates/rubix-prefs/src/apply.rs) `FieldSpec::Measure` does — the
new part is sourcing the field→quantity map from the chart config instead of a
static DTO declaration. **Conversion (and datetime formatting) is a strictly
post-cache, post-batch, per-caller layer:** the cache and the batch snapshot hold
**raw canonical (metric) values**; convert and format *after* reading, per the
requesting principal. Never store converted values — they'd be wrong for the next
caller and would force the unit system into the cache key (§4).

**Datetime → frontend formatting with a backend-supplied pattern.** Do **not**
pre-format timestamps server-side for charts: the x-axis needs the numeric UTC
instant to scale, pick ticks, and drag-zoom. Instead the backend returns the raw
UTC instant, and the client formats it with the principal's `DateTimePattern`
(strftime) delivered alongside prefs. (Non-chart DTOs — tables, audit views —
*can* use `apply_to`'s `FieldSpec::Timestamp` server-side; only charts need the
raw instant.)

**Wiring required (none exists today):**
- **Backend:** `GET/PATCH /prefs` (per-principal), and fold prefs into the query
  path so values are converted per the caller. Store prefs as a principal-scoped
  record so they ride the gate/audit like everything else.
- **Frontend:** a `usePreferences()` hook + context; thread it into the axis/value
  formatters ([charts/utils.ts](../../ui/src/components/chart-builder/charts/utils.ts),
  [format-value.ts](../../ui/src/components/chart-builder/charts/format-value.ts)),
  retiring the hard-coded `en-US`.

**Gap to close — timezone.** `rubix-prefs` has *no* timezone field
([pattern.rs](../../crates/rubix-prefs/src/datetime/pattern.rs) is strftime-only,
always UTC). A dashboard showing "last 1h" almost certainly wants *local* time.
Add an IANA timezone to `Preferences` and apply it at the **client** formatter
(UTC instant + tz + pattern → label). Until then, label axes UTC and say so.

---

## 3. Bulk query API — **add `POST /query/batch`**

Batch's first-order win is collapsing N round trips into one and giving every panel
**one identical data snapshot** (consistency). Be honest about two things the first
draft got wrong:

- **"Build once" is mostly the cache's win, not batch's.** Once §4's scoped-context
  cache exists, N parallel single-panel queries in a tick already share the built
  context. Batch's *durable* value is round-trips + snapshot consistency, not
  rescans.
- **Batch is all-or-nothing latency.** The board paints nothing until the slowest
  of N queries returns, whereas per-panel requests + `keepPreviousData` let each
  panel paint as it lands. If progressive paint matters more than atomic
  consistency, keep per-panel requests and get consistency from a shared cached
  snapshot (§4) instead. Ship batch for the round-trip win — don't treat it as pure
  upside.

```
POST /query/batch
{ "queries": [ { "key": "panel-abc", "sql": "…", "time": {…} }, … ] }   // ≤ ~50

200 { "results": [
  { "key": "panel-abc", "rows": [ … ], "columns": [ … ] },
  { "key": "panel-def", "error": "…" }      // per-item failure; batch still 200
] }
```

- **Per-item errors** so one bad panel doesn't blank the board.
- **Keyed, order-independent** — client matches `result.key → panel`.
- **Same guard, same scoped session, same capability check** per statement
  ([guard.rs](../../crates/rubix-query/src/query/guard.rs)) — batching is
  transport, never a permission shortcut.
- **Return `columns` too** (like `nexus`'s `QueryResponse`), so the client gets
  types without sniffing rows — feeds FieldConfig matching (§7).
- A board issues **one** batch request keyed by `chart_id`; keep single-panel
  `runQuery` for the console/editor.

---

## 4. Optimised query API — **context cache + saved queries + schema endpoint**

Two "repetition" costs hide in "a lot of repeated fields":

**(a) Repeated full-table scans across ticks** — the expensive one, and where the
first draft had a **security bug**. ⚠ The obvious key `(namespace, table-set,
normalised-sql)` is **unsafe**: `scan_table` reads through the principal's
gate-issued **scoped session**, so two principals running identical SQL must see
different rows — a results-by-SQL cache would serve principal A's rows to B, a
permission bypass. **Fix and better design in one:** cache the **per-principal
scanned context** (the scoped `MemTable`s), keyed on `(scope-identity, table-set,
resolved-time-snapshot)`, and run each SQL **fresh** on top. This (i) closes the
cross-principal leak, (ii) holds **raw canonical values** so unit
conversion/formatting stays a post-cache per-caller layer (§2) — never cache
converted numbers or you'd have to key on unit system too, and (iii) is reusable
across *different* SQL hitting the same tables, the bigger win than memoizing
results. **Bound it: LRU + size cap** (a normalised-SQL result cache explodes in
cardinality). Align TTL to the poll snap (§6) so a tick is a guaranteed hit, and
**share the live-event signal** — a write on the §6 invalidation channel must evict
the affected scope/table context, or the board stays stale until TTL despite fresh
data. This *is* the "optimised query api": not new SQL, but **not recomputing the
same answer**, safely.

**(b) Repeated SQL boilerplate** — every chart hand-writes `json_get(content,…)`,
casts, the same `WHERE created BETWEEN …`. Pay it down cheaply, not with the
deferred structured-SQL compiler:
- **Saved queries** as `kind:"query"` records — a chart references a query id
  instead of re-embedding SQL. De-dupes at the record level, and **sidesteps the
  SQL-normalisation minefield** for §4a cache keys (key on query-id + params, not
  whitespace-/order-normalised SQL) — a reason to land saved queries *with* the
  cache. ⚠ A saved query resolves to SQL and then runs under the **caller's** scope
  and caps, never the author's — it must not become a privilege-escalation handle.
- **`GET /query/schema`** — tables + columns the principal can read, row-perm
  aware, for autocomplete and to stop charts guessing the JSON shape.
- **Consider `nexus`'s kind-mode** later — a named, parameterised query id the
  backend resolves to SQL. It's the safe, bounded slice of the no-SQL builder
  (params validated against a schema) without the full spec→SQL translator. Adopt
  if a no-SQL authoring need is voiced; until then raw SQL + saved queries cover
  ~90%.

Still **deferred** (LAMINAR-BORROW §1c/§9): per-`kind` virtual tables and the full
structured query builder.

---

## 5. Time correctness & bucketing — **UTC windows, epoch-aligned buckets, server-side**

**The current bug is timezone.**
[board-params.ts](../../ui/src/components/dashboards/board-params.ts) does
`format(start, 'yyyy-MM-dd HH:mm:ss.SSS')` — **local wall-clock, no tz** — and
splices it into SQL that gets `CAST(… AS TIMESTAMP)`. But `created` is stored as
**UTC** microseconds. A browser at UTC+10 asking for "last 1h" compares a local
window against UTC data — off by the offset, wrong for everyone not at UTC.

**Decision.** Time becomes a structured, UTC, backend concern:
- **Wire format:** client sends absolute UTC instants (epoch ms) + grain, never a
  locale string — mirror `nexus`'s `{ time_range, interval_secs }`:
  ```
  { "sql": "…", "time": { "from": 1718500000000, "to": 1718586400000, "grain": "hour" } }
  ```
- **Backend owns the window + bucket**, reusing the rule layer's epoch-aligned
  grains (`Grain::{Minute,Hour,Day,Week}`,
  [aggregate/mod.rs](../../crates/rubix-query/src/aggregate/mod.rs)) so a chart
  bucket and a rule bucket line up exactly — no `date_trunc` whose week alignment
  differs.
- **Relative tokens, resolved server-side** (borrow `nexus`'s `now-6h`/`now/d`
  grammar and the old project's dual-track model
  ([use-query-time.ts](../../../rubix-old/ui/src/features/time/use-query-time.ts))):
  store the token in board/URL state, resolve to UTC at query time so "last 1h"
  stays fresh across reloads/polls.
- **Auto interval — backend owns the snap.** The client sends the window + a
  **target point count** (~200–300); the **backend** snaps to its `Grain`
  alignment and buckets. Don't recompute the grain table client-side — that
  duplicates the backend's alignment in a second place that will drift. One source
  of truth; feeds §1's downsampling.

`{{…}}` substitution stays for the **raw SQL console**; the **board path** uses
the structured `time` field.

---

## 6. Polling / refresh / live — **visibility-aware timer now, live-invalidation later**

- **Refresh control** on the board (`Off · 5s · 10s · 30s · 1m · 5m`), driving
  TanStack `refetchInterval` (borrow `nexus`'s `useAutoRefresh` /
  [refresh-control.tsx](../../../rubix-old/ui/src/features/time/refresh-control.tsx)).
- **Pause on hidden tabs**, catch up on return.
- **Snapped cache keys** — quantise the range to the refresh tick before it enters
  the query key (`tick:snappedFrom:snappedTo:grain`), so relative "now-1h" doesn't
  bust the cache every render. Snapping is also what makes §4(a)'s server cache
  hit. Use `keepPreviousData` so a refresh doesn't flash a spinner.
- **One batched refresh per tick** (§3), not N.
- **Live later, as debounced *invalidation*** (LAMINAR-BORROW §2): coalesce
  `/ws/records` events over ~1–2s and re-run the panel query once — never a
  re-query per event. `nexus`'s SSE rolling-window streaming is the further
  upgrade for genuine live panels, behind the same pure renderer.

---

## 7. Grafana parity — **adopt the `nexus` data model; keep recharts**

The current `{ type, x, y, breakdown }` config is an MVP, not Grafana. The
`nexus` model already *is* Grafana-shaped and is **library-agnostic** (pure data),
so adopt it as the target chart contract and grow the config into it:

- **FieldConfig** — `defaults` + per-series `overrides` matched `byName`/`byRegex`:
  unit, decimals, min/max, `noValue`, **multi-step threshold ramps**, **value
  mappings** (value/range/regex → text+colour). This is the bulk of "Grafana
  power."
- **Unit registry** — adopt `nexus`'s 30+ unit ids, but back the *physical*
  quantities with `rubix-prefs` conversion (§2) so units aren't just labels.
- **Panel options** — legend placement, y-axis soft min/max, **log scale**.
- **Transforms** — the rename/calc/filter/groupBy/reduce spec, executed
  **server-side** per §1.
- **Template variables** — dashboard-scoped vars resolved into queries
  (`nexus`'s `variables` + the old project's resolution model).
- **Architecture to copy verbatim:** the **pure option-builder + registry**
  pattern (`catalog.ts` descriptors, `renderMap`, `renderWidget` applying
  transforms then dispatching) and the **pure renderer** discipline (a widget is a
  pure function of `{ widget, data }`; one place fetches). This is what makes a
  recharts swap a contained job and keeps widgets testable.
- **Keep recharts as the base — with eyes open on cost.** Your stated preference,
  and it keeps continuity with the stage-1 Laminar chart-builder (already recharts).
  Honest tradeoff: the port rewrites ~8 working, tested `nexus` ECharts builders
  into recharts trees — real effort and bug-risk — though the pure logic
  (FieldConfig resolution, palette via CSS vars, `formatValue`, cartesian chrome)
  ports unchanged. The bundle argument **survives** the §8 ECharts island *because
  the island is lazy-loaded*: common-case boards (line/bar/area/pie/scatter/stat/
  status/table) ship **recharts only**; ECharts loads solely for boards using gauge
  or heatmap. The alternative — adopt `nexus`'s ECharts widgets wholesale and drop
  the recharts base — saves the port but means standardising the *whole* dashboard
  (stage 1 included) on ECharts. Recommendation: stay recharts; revisit only if the
  port effort proves to outweigh the common-case bundle win. Gauge/heatmap are the
  hard ones either way (§8).

This supersedes the four-type Laminar config as the **target**; the Laminar
chart-builder is the shipping MVP we evolve, not a dead end.

---

## 8. Stage 2 — port the `nexus` widget set to recharts

A second stage, after the §1–§7 spine. Ten widgets
(`/home/user/code/rust/starter/nexus/ui/src/features/widgets`), each a pure
function over the shared data/config contract. Port effort by widget:

| Tier | Widgets | Notes |
| --- | --- | --- |
| **Easy** | line, area, bar, pie | recharts' core; near drop-in builders |
| **Medium** | scatter, status, table | status/table are DOM, not charts; scatter needs a numeric x-axis |
| **Hard** | **gauge**, **heatmap** | no recharts equivalent |

- **Reuse unchanged:** the data model (`types.ts`), the registry (`catalog.ts`,
  `renderMap`, `renderWidget`), and all `_shared` pure logic — `fieldConfig.ts`
  (resolution), `palette.ts` (theme via CSS vars + `invalidateThemeCache`),
  `formatValue.ts`, `cartesianChrome.ts`, `scalar.ts`, `thresholdState`,
  `rampColor`. Only the per-widget **option builders** and the **EChart wrapper**
  are rewritten.
- **Gauge / heatmap — thin ECharts island (decided).** No recharts equivalent
  (ECharts' radial gauge and `visualMap` heatmap have no twin), so keep ECharts
  loaded *only* for these two, behind the same pure option-builder + `EChart`
  wrapper `nexus` already uses — their builders port over essentially unchanged.
  Everything else is recharts. Accept two chart libs in the bundle as the price of
  fast parity; revisit a pure-SVG gauge only if bundle size bites. Tree-shake/lazy
  ECharts so boards without a gauge/heatmap don't pay for it.
- **Keep `nexus`'s good discipline:** never fabricate rows (empty → `<Empty>`, not
  a fake zero), live/query behind one `PanelHost`, and `keepPreviousData` refresh.

---

## 9. Build order

Backend and frontend interleave; each step ships independently.

1. **`POST /query/batch`** + per-item errors + `columns` in the response (§3). One
   request per board; context built once. *Biggest win, smallest change.*
2. **Structured UTC `time` field + epoch-aligned bucketing + relative tokens**
   (§5). Fixes the real bug; aligns chart and rule buckets.
3. **Visibility-aware polling + snapped cache keys** (§6).
4. **`GET/PATCH /prefs` + `usePreferences()` + formatter wiring** (§2). Units
   convert server-side; datetime formats client-side. Add the **timezone** field.
5. **Context/result cache, TTL-keyed to the poll snap** (§4a).
6. **Adopt the FieldConfig model** (§7) — thresholds, value mappings, unit
   registry, overrides, log scale. The leap to "Grafana power."
7. **Server-side transform spec** (§1) + **saved queries / `GET /query/schema`**
   (§4b).
8. **Stage 2: widget port to recharts** (§8) — easy tier first; gauge/heatmap stay
   on a lazy-loaded ECharts island.
9. **Row caps / pagination, then downsampling** (§1) — when row counts bite.

Deferred (voiced-need only): full structured no-SQL builder, per-`kind` virtual
tables, kind-mode authoring. Cut (LAMINAR-BORROW §4): the template/custom-widget
renderer until the sandbox is a funded security project.

---

## 10. Open questions

*Decided:* gauge & heatmap stay on a **lazy-loaded ECharts island**, everything
else recharts (§7, §8). The query cache is keyed by **scope identity** (per
principal) and holds **raw values**; conversion/formatting are post-cache (§2, §4a).
**Backend owns** time windowing + interval snap (§5). Transforms are a **portable
spec, hybrid execution**: aggregate ops server-side, cosmetic ops client-side via
nexus's executor (§1).

1. **Transform spec shape** (execution decided hybrid, §1) — the exact stored spec
   (adapt nexus's transform types) and the precise boundary: which ops push into
   SQL vs. a DataFusion post-stage, and the client cosmetic set.
2. **Backend time mechanism** (decided: backend owns the window + interval snap,
   §5) — remaining detail: rewrite the SQL string to inject `WHERE`/bucket, or
   build it into the logical plan?
3. **Unit metadata source** — chart FieldConfig declares each column's `Quantity`;
   how is that authored (a dropdown per series) and where does the
   column→quantity map live so the query endpoint can apply prefs?
4. **Timezone** — add an IANA tz to `rubix-prefs` now (charts want local time), or
   ship UTC-labelled and follow up? (Recommend now — it's small and the bug is
   user-visible.)
5. **Prefs delivery** — bundle prefs into the auth/me response, or a dedicated
   `GET /prefs`? (Former saves a round trip; latter is cleaner to PATCH.)
6. **Variables scope** — dashboard-level only, or also global/org vars (the old
   project's multi-site resolution)?
7. **Batch size & cache TTL** — per-request cap and cache lifetime, tied to the
   poll snap so a tick is a guaranteed hit.
8. **Downsampling method** — LTTB (shape-preserving) vs. a `date_bin` mean, and
   automatic-above-threshold vs. explicit chart option.
