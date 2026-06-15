# BORROW FROM LAMINAR — analytics, dashboards, charts, trace→eval

A grab-bag-but-organized catalogue of what we can lift from **Laminar**
(`lmnr-ai/lmnr`, cloned at `/tmp/lmnr`) into Rubix to get a real
**SQL → chart → dashboard** pipeline, a **trace → evaluation** flow, **custom
render templates / widgets**, and a much better chart layer.

This doc is deliberately maximal — capture everything plausible, decide later.
Nothing here is committed scope; it's a menu. Where an item maps cleanly onto an
existing Rubix contract it's marked **✅ fits**; where it fights our model it's
marked **⚠ adapt** with the friction noted.

> **Review pass (corrections applied).** A first draft overstated three things;
> tightened here. (1) The template sandbox is **not** a solved security boundary
> — Laminar loads deps from esm.sh, allows `unsafe-eval`, and sets
> `sandbox="allow-scripts allow-same-origin"`; treat it as a prototype (§4).
> (2) The DataFusion surface exposes canonical tables with a `content` **JSON
> string** column, not typed per-kind columns, so the chart builder needs a
> catalog/virtual-table layer first (§1). (3) "Everything is a record / no new
> tables" holds for config + artifacts but **not** high-volume facts and
> rollups, which SCOPE's scale path puts in explicit rollup surfaces (§7).
> Frontend is **reference**-portable (a port/refactor), not copy-paste.

**Source authority for us stays** [SCOPE.md](../SCOPE.md) /
[STACK-DEISGN.md](../../STACK-DEISGN.md). Laminar is a reference implementation,
not a target architecture — we borrow *frontend code* freely and *backend ideas*
selectively, because their stack and ours diverge (below).

---

## 0. Stack reality check (what's portable vs. what's only an idea)

| Layer | Laminar | Rubix | Consequence |
| --- | --- | --- | --- |
| Frontend | Next.js 16 / React 19, Zustand, Tailwind v4, Radix, SWR, lodash, framer-motion, react-resizable-panels | Vite / React 18, TanStack Router+Query+Table, Tailwind v4, Radix (shadcn-vendored) | **Reference-portable — a port/refactor, not copy-paste.** Strip Next `app/` routing + server actions + SWR; re-wire to TanStack Router + TanStack Query + our REST client; pull in the transitive deps the components assume (§8). Budget real refactor time per component. |
| Charts | **recharts 3.x** + custom wrappers | **custom hand-rolled SVG** (`ui/src/components/viz/{Line,Bars,Donut,Spark}.tsx`) | Replace SVG with recharts + their wrappers. Biggest single UX win. |
| SQL editor | CodeMirror (`@uiw/react-codemirror` + `@codemirror/lang-sql`) | none | Lift wholesale; swap dialect/schema to ours. |
| Dashboard layout | `react-grid-layout` | none | Lift wholesale. |
| Custom widgets | sandboxed-iframe JSX (Preact + `@twind/core`, CSP-locked) | none | Lift the renderer; the CSP sandbox is the hard, done part. |
| Query backend | Python **query-engine** microservice (JSON↔SQL + validator) over **ClickHouse** | **DataFusion** (`rubix-query`, `POST /query`) over **SurrealDB** | Don't port Python. Re-implement the *JSON-query-spec + validator* idea inside `rubix-query`. |
| Operational store | Postgres (metadata) + ClickHouse (analytics) | **SurrealDB** (single store: doc+graph+vector+TS) | Their dual-DB split is an idea, not a port. SurrealDB plays both roles; DataFusion does the heavy aggregation. |
| Trace store | OTel spans → ClickHouse `spans`/`traces_replacing` | `rubix-trace` spans on the bus → SurrealDB `trace` table (sampled) | We already have the ingest half. Borrow the *rollup + typing + eval* model. |
| Saved queries / charts / dashboards / templates | dedicated Postgres tables | **records** (`kind:"query"|"chart"|"board"|"template"`) — our collections model | ✅ fits perfectly. No new tables; everything is a tagged record. |

**One-line takeaway:** port the *frontend* as a reference (refactor, not paste);
re-implement the
*query/validate* layer thin on top of DataFusion; treat the trace/eval/dataset
*data model* as the thing worth copying on the backend. Persist every new
artifact (query, chart, board, template, evaluation) as a **record** so it rides
the gate, audit, scoped-session, and live-query machinery we already built.

---

## 1. The SQL → chart → dashboard pipeline (headline feature)

We have **none** of this today: no query console, no chart builder, no
dashboards. ADMIN-UI specs a `<QueryConsole>` and PRODUCT-UI specs pinned boards
— Laminar has both, better than the spec, and portable.

Three stacked layers; each is independently useful, so we can land them in order.

> **⚠ Prerequisite — a typed query catalog must come first.** Laminar's
> `QueryStructure` (typed columns, `quantile(q)(col)`, `toStartOfInterval`,
> `WITH FILL` gap-fill) assumes a columnar store with real typed columns.
> Rubix's DataFusion surface exposes a handful of **canonical tables with a
> `content` JSON *string* column** ([schema.rs](../../crates/rubix-query/src/provider/schema.rs)) —
> fields are reached with JSON functions, never flattened. So before a usable
> chart builder we need a **catalog layer**: collection-backed **virtual tables /
> typed projections** — one DataFusion table (or view) per `content.kind`, with
> the typed fields projected from that collection's definition — plus DataFusion
> equivalents for the time-bucket / gap-fill / quantile functions the builder
> emits. Until that exists, only the *raw SQL* editor (§1a) is truly drop-in; the
> structured builder (§1c) is gated on the catalog. This is the real first
> backend task, not the editor port.

### 1a. SQL editor / query IDE — `frontend/components/sql/`
- `editor-panel.tsx` — full IDE: **Query / Results / Parameters / Chart** tabs in
  resizable panels. This *is* our `<QueryConsole>`, done.
- `sql-editor.tsx` — CodeMirror wrapper: SQL syntax, autocomplete, search, an
  "Ask AI → generate SQL" button (calls a generate endpoint with the current
  query as context).
- `utils.ts` (~886 lines) — CodeMirror setup + a **schema-aware completion
  source** (their ClickHouse dialect). **Swap the schema** for our SurrealDB
  tables + DataFusion column catalog → schema-aware autocomplete over the record
  graph. Completion quality tracks the catalog layer (⚠ prerequisite above): with
  only canonical tables it autocompletes table names + structural columns; typed
  per-kind fields appear once virtual tables/projections exist.
- **Parameters** (`sql-editor-store.ts`): `start_time` / `end_time` /
  `interval_unit` template params substituted into the query. Maps directly onto
  our `POST /query` time-window bucketing (`minute…week` epoch-aligned).
- **Saved templates**: debounced auto-save + CRUD list (`sidebar.tsx`). In Rubix
  these are `kind:"query"` records. ✅ fits.

**Port plan (1a):**
1. `pnpm add @uiw/react-codemirror @codemirror/lang-sql @codemirror/autocomplete`
   into `rubix/ui`.
2. Copy `components/sql/` → `ui/src/components/sql/`. Delete the Next server-action
   imports; point fetches at `api/query.ts` (`POST /query`) and a new
   `api/queries.ts` (CRUD over `kind:"query"` records).
3. Replace the ClickHouse completion schema in `utils.ts` with a Rubix catalog —
   ideally fetched from a new `GET /query/schema` (tables + columns the principal
   can read) so autocomplete respects row-perms.
4. Defer the "Ask AI" button — it lands naturally once `rubix-agent`'s brain is
   wired (§5); stub it disabled for now (mirrors PRODUCT-UI's disabled-action
   pattern).

### 1b. Chart builder (query result → chart) — `frontend/components/chart-builder/`
The config model is tiny and clean (verified from source):
```ts
enum ChartType { LineChart="line", BarChart="bar", HorizontalBarChart="horizontalBar", Table="table" }
type DisplayMode = "total" | "average" | "none";
interface ChartConfig {
  type?: ChartType; x?: string; y?: string;
  breakdown?: string;            // multi-series split column
  displayMode?: DisplayMode;     // headline metric over the series
  tableColumnConfig?: { columnOrder?; columnSizing?; columnVisibility? };
}
```
- Zustand store persisting to a storage key (`chart-builder-store.tsx`) — drop-in.
- **`charts/utils.ts` is the gold** — take it whole:
  - `getOptimalDateFormat()` — picks `M/dd` vs `M/dd HH:mm` vs `HH:mm` from point
    density.
  - `transformDataForSimpleChart()` / `transformDataForBreakdown()` — flat rows →
    recharts series (incl. pivot to multi-line).
  - `createAxisFormatter()`, compact number formatter (`1.23K`), null-gap handling,
    UTC timestamp parsing.

### 1c. Structured query builder (no SQL) — `frontend/components/dashboards/editor/fields/`
Visual builder over a persisted JSON spec (verified `QueryStructure`, zod):
```ts
QueryStructure = {
  table: string,
  metrics: { fn: "count"|"sum"|"avg"|"min"|"max"|"quantile"|"raw", column, args:number[], alias?, hidden? }[],
  dimensions: string[],
  filters: { field, op:"eq"|"ne"|"gt"|"gte"|"lt"|"lte", stringValue|numberValue }[],
  timeRange?: { column, from, to, intervalUnit?, intervalValue?, fillGaps },
  orderBy: { field, dir:"asc"|"desc" }[],
  limit?: number,
}
```
Field components (≈1200 lines total): `ChartTypeField`, `MetricsField`
(count/sum/avg/min/max/**p90/p95/p99**/raw), `DimensionsField`, `FiltersField`,
`OrderByField`, `LimitField`.

**The load-bearing idea worth copying:** persist the **structured spec alongside
the generated SQL**, with a SQL→spec parser fallback. A chart round-trips
edit→save without re-writing SQL by hand. The spec is the source of truth; SQL is
a projection.

**Backend for 1c — re-implement, don't port.** Laminar's Python `query-engine`
does `json_to_sql.py` / `sql_to_json.py` / `query_validator.py` (injection guard +
table/column whitelist + project-id match). We build the equivalent **inside
`rubix-query`**:
- `QueryStructure` (JSON) → DataFusion `LogicalPlan` / SQL string.
- ⚠ **Function translation, not reuse.** The spec's aggregates/time ops are
  ClickHouse-shaped (`quantile(q)(col)`, `toStartOfInterval`, `WITH FILL`). Map
  each to a DataFusion equivalent — `approx_percentile_cont` for quantiles,
  `date_bin` for bucketing, and an explicit calendar/spine join for gap-fill
  (DataFusion has no `WITH FILL`). This translation table is part of the work.
- Validation reuses our **scoped session**: the whitelist *is* the principal's
  readable tables/columns; the tenant match *is* the namespace. ⚠ adapt: we get
  the security property from the gate, not a bespoke validator.
- Expose `POST /query/from-spec` (spec → rows) and `POST /query/to-spec`
  (SQL → spec, best-effort) next to the existing `POST /query`.

**Port plan (1b+1c):**
1. `pnpm add recharts zustand date-fns` into `rubix/ui`.
2. Copy `chart-builder/` and `dashboards/editor/` into `ui/src/components/`.
3. Keep `charts/utils.ts` verbatim; only the data-fetch boundary changes.
4. Add `api/queries.ts` (CRUD `kind:"query"`) and the two new `rubix-query`
   endpoints. Until `to-spec` exists, default new charts to the structured builder
   so the spec is always authored, never parsed.

---

## 2. Dashboard builder — `frontend/components/dashboards/`
- `grid-layout.tsx` — `react-grid-layout` responsive 12-col grid, drag-handle,
  resize, **500ms-debounced PATCH** persistence. This is PRODUCT-UI's pinned
  boards.
- `DashboardChart` shape (verified):
  ```ts
  { id, name, query, createdAt,
    settings: { config: ChartConfig, layout:{x,y,w,h}, queryStructure? } }
  ```
  → in Rubix, a board is a `kind:"board"` record holding an ordered list of
  `kind:"chart"` record ids + their layout; each chart record holds
  `{ query, config, queryStructure }`. ✅ fits our collections model exactly;
  boards persist server-side (resolving PRODUCT-UI's localStorage gap and the
  AGENT.md "board persistence" open question — it's just a gate write).
- `chart-presets.ts` + `add-chart-dropdown.tsx` — preset templates. Ours:
  "p95 latency", "error rate over time", "readings per source", "insights per
  rule", "audit volume per principal".

**Port plan:** `pnpm add react-grid-layout`; copy `dashboards/`; re-point the
PATCH to `api/boards.ts` (CRUD `kind:"board"`). Debounced save → one gate command
per settle, so every layout change is audited + undoable (free, via WS-05/06).

**Worth flagging (with a caveat):** because boards/charts/queries are records,
they ride the **live-query bus** (WS-07) — a dashboard can subscribe to a
`kind` and refresh on change instead of polling on a timer like Laminar does.
**But: use live events as a debounced *invalidation* signal, not a re-query per
DataChange.** A live `kind:"reading"` stream can fire thousands of events/sec;
re-running an aggregation query on each would melt the panel. Coalesce events over
a window (e.g. 1–2s) and re-run the chart's query once. Done that way it's a real
improvement over timer-polling; done naively it's worse.

---

## 3. Charts & chart options (explicitly requested — "their charts are really good")

Replace `ui/src/components/viz/*` (custom SVG) with **recharts + Laminar's
wrappers**. The library matters less than the four wrapper behaviours that make
their charts feel good — these are the parts we'd otherwise reinvent badly:

1. **Drag-select-to-zoom** — `ReferenceArea` overlay captures a time range on
   `line-chart.tsx`; emits start/end, re-queries the window.
2. **Cross-chart sync** — shared `syncId` ties tooltips/cursors across every chart
   on a board (hover one, see the same instant on all).
3. **Click-to-drill-down** — `onBarClick` / point click opens the underlying
   row/trace. Their `hidden` metric flag (e.g. `trace_id`) is selected in SQL but
   hidden from the table, purely as a click target. Ours: click a reading → open
   its record / correlation-id trace.
4. **Smart time-axis** — `getOptimalDateFormat()` (see §1b).

Chart components to take: `charts/{line,bar,horizontal-bar,table}-chart.tsx`,
`charts/index.tsx` (`ChartRendererCore` dispatch), `components/ui/chart.tsx`
(`ChartTooltipContent`, CSS-var palette `--chart-1..5`).

`table-chart.tsx` is **TanStack Table** (we already use it): infinite scroll,
persisted column visibility/sizing/ordering, JSON truncation, date formatting.
Low-friction.

Chart-type / option coverage to match:

| Option | line | bar | hbar | table |
| --- | --- | --- | --- | --- |
| x / y column | ✓ | ✓ | ✓ | — |
| breakdown (multi-series) | ✓ | ✓ | — | — |
| displayMode total/avg | ✓ | ✓ | ✓ | — |
| stacked series | ✓ | ✓ | — | — |
| column vis/size/order | — | — | — | ✓ |
| drag-zoom / sync / drill | ✓ | ✓ | ✓ | n/a |

**Nice-to-have beyond Laminar (note for later):** area, scatter, heatmap (great
for per-sensor-over-time), gauge/stat (we have `StatCard`), and a geo/map layer
(SurrealDB is geospatial — no Laminar equivalent, but our domain wants it).

---

## 4. Render templates / custom widgets (explicitly requested)
- `components/ui/template-renderer/jsx-renderer.tsx` — user-authored **JSX/Preact
  rendered in an iframe**, Tailwind via `@twind/core`. This is exactly "custom
  widgets authored from the frontend" — but see the security note below; their
  iframe is **not** a hardened boundary.
- `template-picker.tsx` + `settings/render-templates/` — template CRUD, **live
  preview against test data**. Shape: `{ name, code, testData }` → `kind:"template"`
  record. ✅ fits.
- `components/ui/content-renderer/` — multi-mode field viewer
  (json/html/markdown/code/base64-image) with a template hook, for rendering a
  single cell/field with a custom view (e.g. a reading payload, a rule result).

**⚠ Security: borrow the shape, NOT the boundary.** Laminar's renderer is a
useful *prototype*, not a solved sandbox. As shipped it loads deps from
**esm.sh** (network egress), enables **`unsafe-eval`**, and runs the iframe with
**`sandbox="allow-scripts allow-same-origin"`** — and `allow-same-origin` +
`allow-scripts` together means the frame can reach back into the parent origin, so
a malicious template *can* exfiltrate. That's fine for their single-tenant trust
model; it is **not acceptable for Rubix multi-tenant cloud**. Before we let
operators author widgets we must harden it:
- **Self-bundle** the template's deps (no esm.sh / no runtime CDN fetch).
- **Drop `allow-same-origin`**; render on a null origin so the frame can't touch
  the parent.
- **Strip network** entirely via CSP (`default-src 'none'`), and avoid
  `unsafe-eval` (precompile JSX → JS at save time, ship the compiled module).
- **iframe sizing via `postMessage`**, not same-origin DOM measurement.

This hardening is the actual work of §4 — the picker/CRUD/preview UI is the easy,
portable part. Put templates **last** in the take-order until the sandbox is real.

**Port plan:** `pnpm add @twind/core preact`; copy `template-renderer/` +
`content-renderer/`; back with `api/templates.ts` (CRUD `kind:"template"`).
Where Laminar feeds templates from ClickHouse rows, we feed from `POST /query`
rows or a record's fields.

**Tie-in:** a board widget can be `type:"template"` — the dashboard renders the
user's JSX with the query result as props. That unifies §1–§4: **query → (chart |
template) → board.**

---

## 5. Trace → Evaluations flow (explicitly requested) — and the agent angle

This is where Rubix already has the *ingest* half and Laminar has the *flow on
top*. And critically: **`rubix-agent` exists** (scoped principal, tiered
analyst⊂operator⊂actuator, vector memory) but its **LLM brain/run loop is not
wired yet** (Rig adapter planned, per `crates/rubix-agent/src/lib.rs` + AGENT.md).
Agent *runs* are precisely the thing a trace→eval system observes — so adopting
Laminar's model now shapes how we wire the brain later.

What we have (`rubix-trace`, WS-08): correlation-id spans on the bus, `parent_span_id`
trees, sampled append-only `trace` table, `assemble_trace`/`SpanNode` read-back.

What to borrow from Laminar's model:

### 5a. Span typing as a first-class enum
Laminar spans carry a type: `DEFAULT / LLM / TOOL / EVALUATOR / EVALUATION /
HUMAN_EVALUATOR / PIPELINE / EXECUTOR`. Our spans carry a `work` step name.
**Promote `work` → a typed `span_kind`** so we can render/filter per type (LLM
calls vs. tool calls vs. rule evals) and compute per-type cost/latency. ⚠ adapt:
ours is generic, so keep an open `Other(String)` arm.

### 5b. Trace-level rollup at ingest
`app-server/src/db/trace.rs::upsert_trace_statistics_batch`: as child spans land,
upsert onto a parent **trace summary** row — status = error if any child errored,
summed cost/tokens, distinct tags, span count, top-span name/type. So the trace
list is **one cheap read**, not a tree walk. **Direct borrow for `rubix-trace`:**
maintain a `trace_summary` per correlation id, updated on span persist. Our
edge-partitioned append-only model makes this a natural fold.

> **⚠ Contract change first.** This needs a richer span than we have. Today
> [`Span`](../../crates/rubix-trace/src/span.rs) carries only `name`,
> free-form `attributes`, and start/end ns — **no typed `span_kind`, status,
> token, or cost fields**. We cannot roll up cost/tokens/status until those are
> first-class on the span (or reliably parsed out of `attributes`). So §5a (span
> typing + a defined **span metric schema**: kind, status, tokens, cost) is a
> hard prerequisite for §5b — define the schema before promising the rollup.
> And per §7, `trace_summary` is a **rollup surface** (a materialized, versioned
> table), *not* a plain record — it's high-volume derived data.

### 5c. Evaluation as a scored view of a trace
Laminar's `EvaluationDatapointResult`:
```
{ data, target, scores: map<string,f64>, traceId, executorOutput, metadata,
  groupId, datasetLink?:{datasetId,datapointId} }
```
An evaluation = a named, scored assessment of a run, with `groupId` to **compare
runs over time**. Scores aggregate (avg/p90/p95/p99) per time bucket
(`frontend/lib/clickhouse/evaluation-scores.ts`) → plots on our existing
time-window query surface.

**The best conceptual lift: this maps almost 1:1 onto `rubix-rules`.** A rule
evaluation (WS-11) already produces a decision + a per-eval span tree recorded via
the gate. Add a `scores: map<string,f64>` and a `groupId` and **rule-insights
*become* evaluations** — comparable across runs, chartable, dashboard-able. Same
move covers **agent runs**: when the brain is wired, an agent turn is a trace; an
eval of it (did the tool call succeed? was the answer grounded?) is an evaluation
datapoint. One model serves rules *and* agent QA.

Proposal: an evaluation is a `kind:"evaluation"` record linking a `trace_id`,
carrying `scores` + `group_id`, written through the gate (audited, correlated).
✅ fits.

### 5d. Datasets & labeling queues
- **Dataset** = collection of `{ data, target, metadata }` datapoints
  (`kind:"dataset"` + `kind:"datapoint"` records).
- **Labeling queue** (`labeling_queues` + `annotationSchema` JSONB) = human/LLM
  annotation workflow feeding datasets. This is PRODUCT-UI's "attention queue with
  actions," with a concrete schema-driven shape: a queue item presents `payload` +
  an `annotationSchema`; an operator (or the agent as a judge) labels it; the
  label pushes to a dataset. ⚠ adapt: our queue items are records; the annotation
  schema is a JSON Schema (we already do schema-driven forms in ADMIN-UI's
  `<SchemaForm>`).
- **Trace → dataset**: promote any traced run (a good rule fire, a bad agent
  answer) into a dataset datapoint for regression testing later.

### 5e. LLM-as-judge / evaluator definitions
Laminar runs evaluators (LLM-as-judge via "signals" with a sample rate, or
API-supplied scores). For us, an **evaluator is a rule or an agent invocation**
that emits scores — no new engine. `rubix-rules` already runs offline and writes
via the gate; an evaluator is "a rule whose output is a `scores` map." When the
agent brain lands, LLM-as-judge is "an agent invocation in `analyst` tier whose
output is a `scores` map." ✅ fits both engines we already have.

---

## 6. Backend ingestion / storage ideas (selective)
Borrow as *ideas*, not Rust — their storage is ClickHouse-specific.
- **Hash-based dedup of repeated payloads** (`traces/input_dedup.rs`): store a
  large repeated body once, reference by 32-byte hash, JOIN on read (10–100×
  win). Applies to repeated edge readings / repeated prompts. Maps onto SurrealDB
  record links. ⚠ evaluate against SurrealKV first.
- **Batched async inserts with adaptive flush** (~1000 rows / 400ms): pattern for
  `rubix-ingest`'s decimate→persist path (we may already do similar).
- **Version-column dedup for late arrivals** (ClickHouse `ReplacingMergeTree`,
  version = num_spans): late spans don't clobber a complete trace summary. The
  edge↔cloud sync (`rubix-sync`) has the same late-arrival problem — a version
  column on `trace_summary` is a cleaner tiebreak than LWW for the rollup case.
- **Structured-intent-as-source-of-truth + tenant-scoped validation** (§1c):
  already the recommended `rubix-query` shape.

---

## 7. Persistence map — two tiers, not "everything is a record"
**Config + artifacts are records** (✅ our collections model — they ride the gate,
audit, scoped session, live-query). **High-volume facts + derived rollups are
explicit query/rollup surfaces**, per SCOPE's scale path
([SCOPE.md "Scale path"](../SCOPE.md)): pre-aggregated rollup tables written as
data lands, behind the historian/DataFusion boundary. Conflating the two would put
unbounded analytical volume into the record table and onto the live-query bus.

**Tier A — records (`kind`):** config + low-volume artifacts.

| Artifact | `kind` | Holds | Rides |
| --- | --- | --- | --- |
| Saved query | `query` | `{ sql, queryStructure, params }` | gate, audit, scoped read, live-query |
| Chart | `chart` | `{ query_id\|sql, config, queryStructure }` | ″ |
| Dashboard | `board` | `{ charts:[{chart_id, layout}] }` | ″ |
| Render template | `template` | `{ code, testData }` | ″ |
| Dataset / item | `dataset` / `datapoint` | `{ data, target, metadata }` | gate (dataset def is config; large item sets may move to Tier B) |
| Labeling queue / item | `labeling_queue` / `queue_item` | `{ payload, annotationSchema }` | gate |

**Tier B — rollup / fact surfaces (explicit tables, not records):** high-volume or
derived, behind the historian boundary.

| Surface | Holds | Why not a record |
| --- | --- | --- |
| `trace_summary` | per-correlation-id rollup, versioned | high churn, derived; needs late-arrival version dedup (§6) |
| span facts | typed span metrics (kind/status/tokens/cost) | high volume; queried analytically, not edited |
| evaluation datapoints | `{ trace_id, scores, group_id, data, target }` | grows with every run; aggregated over time, not CRUD'd |
| metric rollups | pre-aggregated time buckets | the SCOPE scale lever itself |

*Note:* the **evaluation definition** (name, evaluator, group) can be a Tier-A
record; its **datapoints** are Tier B. Same split as dataset def vs. items.

Implication: BACKEND-COLLECTIONS' kind/tag filtering + the live-query bridge cover
**Tier A**. **Tier B** needs the rollup-surface + catalog work (§1 prerequisite,
§5b). Most *UI* work is frontend; the load-bearing *backend* work is the catalog +
rollup surfaces, not the records.

---

## 8. Dependencies to add to `rubix/ui`
Direct: `recharts`, `react-grid-layout`, `zustand`, `date-fns`, `@twind/core`,
`preact`.
SQL editor (CodeMirror is several packages, not one): `@uiw/react-codemirror`,
`@codemirror/lang-sql`, `@codemirror/autocomplete`, `@codemirror/state`,
`@codemirror/view`, `@codemirror/search`, `@uiw/codemirror-themes`,
`@lezer/highlight`.
Transitive deps the copied components assume (audit each component before paste):
`lodash` (chart/data utils), `d3-scale` (axis scales), `react-resizable-panels`
(editor panel layout), `react-hotkeys-hook` (editor shortcuts), and a
**replacement for SWR + Next server actions** (use TanStack Query + our REST
client — every `useSWR`/`"use server"` call is a rewrite point).
Already present (low-friction reuse): TanStack Table/Query/Router, Radix/shadcn.

---

## 9. Suggested take-order (revised after review)
Reordered so backend prerequisites land before the UI that depends on them, and
the unhardened sandbox goes last.

1. **Recharts wrapper port** (§3) — replaces SVG; drag-zoom/sync/drill. Adapt the
   formatters to Rubix datetime/unit **prefs** (`rubix-prefs`), not Laminar's
   hard-coded locale. The one genuinely near-drop-in win.
2. **Raw SQL editor panel + saved queries as records** (§1a) — `<QueryConsole>`,
   Tier-A records. Works against the existing `content`-JSON surface today.
3. **Query catalog endpoint** (§1 prereq) — `GET /query/schema` backed by
   collection definitions; drives autocomplete and the structured builder.
4. **Collection virtual tables / typed projections in `rubix-query`** (§1 prereq)
   — one DataFusion table/view per `content.kind`; + the function-translation
   table (quantile/bucket/gap-fill).
5. **Structured chart builder over the catalog** (§1b/1c) — `from-spec`/`to-spec`,
   spec-as-source-of-truth.
6. **Dashboards as records** (§2) — live events as **debounced query
   invalidation**, *not* a re-query per reading. (Correction to the earlier
   "stream per DataChange" framing — debounce, don't thrash.)
7. **Span metric schema + typed spans** (§5a) — the contract change that unblocks
   rollups.
8. **Trace rollup surface** (§5b) — Tier-B `trace_summary`, versioned for late
   arrivals.
9. **Evaluation model on `rubix-rules`** (§5c) — rule-insights → comparable,
   chartable evaluations; positions agent-run QA for when the brain lands.
10. **Datasets + labeling queues** (§5d) — regression sets + attention queue.
11. **Hardened template renderer** (§4) — **last**, only after the sandbox is real
    (self-bundled, null-origin, network-stripped, no `unsafe-eval`).

Land 1–6 and Rubix has a real analytics product; 7–10 turn it into an
observability+eval platform for rules *and* the agent; 11 adds custom widgets once
it's safe to.

---

## 10. Open questions / decisions to make later
- **`rubix-query` surface:** add `from-spec` / `to-spec` endpoints, or keep raw
  SQL only and author the spec purely client-side? (Affects whether SQL→spec
  parsing lives in Rust or TS.)
- **Autocomplete schema source:** static client catalog vs. `GET /query/schema`
  honoring row-perms. (Latter is more work, leaks nothing.)
- **Span typing:** how generic? Closed enum + `Other(String)`, or a free tag?
  (We're generic-by-construction; a closed enum fights that.)
- **Evaluation home:** new `kind:"evaluation"` records vs. extend the existing
  insight record shape from `rubix-rules`. (Reuse vs. clarity.)
- **Template trust:** is the iframe+CSP sandbox sufficient for multi-tenant cloud,
  or do we also need server-side review/approval of template code?
- **Dedup (§6):** worth it on SurrealKV, or premature? Measure first.
- **Geo/heatmap charts:** beyond Laminar — do we invest, given SurrealDB
  geospatial + a sensor domain that wants maps?
- **Agent "Ask AI → SQL" + LLM-as-judge:** both depend on the unwired brain (Rig
  adapter). Stub disabled now (PRODUCT-UI pattern), wire when the brain lands.

---

## 11. Source file index (in `/tmp/lmnr`, for whoever ports)
```
frontend/components/sql/{editor-panel,sql-editor,sidebar,template-editor}.tsx
frontend/components/sql/{utils.ts, sql-editor-store.ts, parameters-panel.tsx}
frontend/components/chart-builder/{index,types,chart-builder-store}.tsx
frontend/components/chart-builder/charts/{index,line-chart,bar-chart,
  horizontal-bar-chart,table-chart}.tsx
frontend/components/chart-builder/charts/{utils.ts, format-value.ts}
frontend/components/dashboards/{grid-layout,chart,add-chart-dropdown}.tsx
frontend/components/dashboards/{types.ts, chart-presets.ts}
frontend/components/dashboards/editor/{Builder,Form,dashboard-editor-store}.tsx
frontend/components/dashboards/editor/{constants.ts, fields/*}
frontend/components/ui/{chart.tsx, template-renderer/*, content-renderer/*}
frontend/lib/actions/sql/types.ts            # QueryStructure (zod)
frontend/lib/clickhouse/evaluation-scores.ts # score-over-time aggregation
query-engine/src/{json_to_sql,sql_to_json,query_validator}.py  # idea, not port
app-server/src/db/trace.rs                    # upsert_trace_statistics_batch (rollup)
app-server/src/traces/{spans.rs, input_dedup.rs, processor.rs}
app-server/src/db/{evaluations.rs, datasets.rs, labeling_queues.rs}
app-server/src/ch/evaluation_datapoints.rs
```
