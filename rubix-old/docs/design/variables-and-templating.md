# Dashboard Variables & Templating

Scope for **dashboard variables**: one dashboard, parameterised. A variable bar lets
a user pick values (`$site`, `$equip`, `$datasource`, `$building`); widgets re-query
against the selection; values deep-link via URL. This is the feature that lets **one
dashboard serve a whole fleet** of sites instead of hand-authoring one board per
site — directly serving the building/energy/HVAC use case rubix targets.

This mirrors the nexus WS-02 design, re-grounded in rubix. The crucial difference:
**rubix has no SQL macro/variable binder yet** — nexus shipped one upstream, rubix
has not. So this scope owns *both* the variable model/UI *and* the server-side
interpolation engine (built injection-safe). It pairs with
[time-range-and-refresh.md](time-range-and-refresh.md) (which supplies `$__from`/
`$__to`) and [page-context-and-nav.md](page-context-and-nav.md) (which adds `context`
as a variable *source*).

## Problem

Today a widget's query is **static SQL frozen at pin time**, with no parametrisation:

- The widget model is `{ id, dashboard_id, site_id, kind, title, target, query?,
  settings? }` (`crates/rubix-core/src/model.rs:147-179`); `query` is a raw string,
  valid only for the `Datasource` widget kind.
- The query route `POST /api/v1/datasources/{id}/query` takes
  `{ sql, params: Vec<Value> }` (`crates/rubix-server/src/api/datasources/run.rs`) —
  positional binds only, **no named-variable interpolation**.
- `POST /api/v1/query` (DataFusion over canonical tables) takes `{ sql }` only
  (`api/query/run.rs`) — no variables at all.
- The UI has **no variable concept** whatsoever (verified: no `variable`/`template`/
  `$var` anywhere in `ui/src`). A widget binds a fixed `target` keyexpr at pin time
  (`ui/src/features/builder/components/widget-binder.tsx`).

Consequence: serving N sites means N near-identical dashboards. There is no
`WHERE site_id = '$site'` that a dropdown can drive.

## Scope

### 1. Variable model

A `Variable` lives in the dashboard's stored config (in the dashboard row's JSON, so
it travels with export/import — do not add a separate relational table; the dashboard
is the unit). Add a `variables: Vec<Variable>` to the dashboard model
(`crates/rubix-core/src/model.rs`, the `Dashboard` struct) and its TS mirror
(`ui/src/api/types.ts`).

```
Variable {
  name:        String,            // "$site" referenced as $site / ${site}
  label:       Option<String>,
  kind:        VariableKind,
  config:      VariableConfig,    // per-kind (tagged)
  current:     Value | Vec<Value>,// selected value(s)
  multi:       bool,
  include_all: bool,
  hidden:      bool,
}
```

**Variable kinds** (a closed enum — adding one is a DTO + UI change, mirroring the
existing `WidgetKind` pattern at `model.rs`):

- `constant` — fixed value, usually hidden.
- `custom` — static comma-separated option list.
- `query` — options come from running SQL (returns one column → option list).
- `datasource` — options are the org's datasources of a given kind, so `$ds` can
  drive which datasource widgets target.
- `site` — options are the org's sites (rubix-native: the natural fleet axis is
  site; `$site` → `site_id`). This is the headline kind for rubix.
- `interval` — a list of durations, drives `$__interval` overrides (pairs with
  time-range).
- `textbox` — free text.
- built-ins (read-only, from context): `$__org`, `$__site`, `$__user`, `$__from`,
  `$__to` (the last two from [time-range-and-refresh.md](time-range-and-refresh.md)).

### 2. Server-side interpolation engine (new — the injection boundary)

Because rubix has no binder, build one in `crates/rubix-query` (shared by both the
DataFusion `/query` path and the datasource `/query` path). Extend both request DTOs
with `variables: Vec<QueryVariable>` where `QueryVariable { name, value(s) }`.

Supported substitutions in SQL text, lowered **before** execution:

- `$name` / `${name}` → single value, bound as a positional/`$N` parameter (never
  string-inlined).
- `${name:csv}` → comma-joined, each value its own bound parameter.
- `${name:singlequote}` → quoted list for `IN`-style use, each value a bound param.
- `$__sqlIn(name)` → safe `IN ($1, $2, …)` expansion from a multi-value variable.

**This is the security boundary.** Every variable value becomes a bound parameter,
never concatenated into SQL — a value of `'); DROP TABLE points; --` binds as a
literal string and cannot execute. Quoting/escaping lives in the engine, server-side,
and is mandatory. Reuse the existing read-only/timeout/row-cap guards already on the
query path (`crates/rubix-server/src/api/query/run.rs`,
`crates/rubix-server/src/api/datasources/run.rs`); do not invent a second guard.

### 3. Variable bar UI

`ui/src/features/variables/**` (new): render each visible variable as a single- or
multi-select (or textbox), with an "All" option when `include_all`. Mount it above
the canvas in the builder (`ui/src/features/builder/index.tsx`).

### 4. Variable editor

In the dashboard settings dialog (alongside
`ui/src/features/builder/components/dashboard-form-dialog.tsx`): add/edit/reorder/
delete variables, choose kind, author the option query, preview resolved options,
set multi/include-all/hidden.

### 5. Resolution & cascading

Resolution lives in `ui/src/features/variables/resolve.ts` (new):

- **Order:** built-ins → constants/custom → site/datasource → query (topological by
  dependency). Resolve on dashboard load and on any parent change.
- **Cascading:** a `query` variable's SQL may reference another variable
  (`WHERE site_id = '$site'`); changing the parent re-resolves the child. Build a
  dependency order; **detect and reject cycles** with a clear error.
- Cache resolved option lists per `(name, parent-values, timeRange)`.

### 6. Re-query on change — fold into the query key

The widget query today keys on `['widgets', siteId, dashboardId]`
(`ui/src/api/keys.ts`) and the data hook polls at `LIVE_INTERVAL`
(`ui/src/api/hooks.ts`). Per-widget *data* queries must additionally key on the
resolved variable values so a selection change re-fetches exactly the dependent
widgets. Introduce a `varRevision` (a hash of resolved variable values) and add it to
the data query key for any widget whose SQL references a variable. A widget with no
variable reference is unaffected (back-compat).

### 7. URL state

`?var-site=Site-A&var-site=Site-B` (repeatable for multi-select); restore on load;
shareable. Rubix currently has **no query-param sync** (scope is URL-path only,
`ui/src/context/scope-provider.tsx`) — this introduces the first one; keep the
`var-` prefix reserved for explicit variable state (see context doc for bare params).

### 8. Repeat-by-variable hook

Expose the resolved value list so a future "repeat this widget per value" feature can
consume it. Out of scope to *render* the repeat here; just expose the list.

## Design notes

- **Variables live in the dashboard JSON, not a table.** Keeps export/import
  self-contained and lets the audit/undo ledger treat a variable edit as part of the
  dashboard snapshot (no double-recording — confirm with
  [audit-and-undo.md](audit-and-undo.md)).
- **Site is the first-class fleet axis.** Where Grafana leans on a generic `query`
  variable, rubix's domain already has `site_id` on every entity
  (`model.rs`), so a `site` kind that resolves to `site_id` is the common path and
  should be the default offered in the editor.
- **All / multi:** support explicit expansion first (`All` → full `IN` list) — it's
  predictable and pushdown-friendly — before any wildcard token.

## What to prove

1. A `query` variable's dropdown is populated from live SQL.
2. A widget using `site_id = '$site'` (or `$__sqlIn(site)`) re-queries when the
   selection changes; an unrelated widget does not.
3. Multi-select + "All" produce correct, safely-quoted `IN (...)`.
4. Cascading: changing `$site` re-resolves `$equip`; a cycle is rejected with a clear
   error.
5. Variable values deep-link via `?var-…` and restore on reload.
6. Injection: a value containing `'); DROP …` binds as a literal, never executes.

## Acceptance criteria

- [ ] `variables` field on the dashboard model + TS mirror; survives export/import.
- [ ] Variable kinds: constant, custom, query, datasource, site, interval, textbox +
      built-ins.
- [ ] Server interpolation engine in `rubix-query`: `$var`, `${var}`, `${var:csv}`,
      `${var:singlequote}`, `$__sqlIn(var)`, every value bound (injection-safe);
      both query paths accept `variables`.
- [ ] Variable bar + editor mounted in the builder; preview resolved options.
- [ ] Resolution order + cascading + cycle detection.
- [ ] `varRevision` folded into the widget data query key; selection change
      invalidates exactly dependent widgets.
- [ ] URL `?var-*` round-trips.
- [ ] Audit/undo: a variable edit is captured under the dashboard kind (confirm with
      [audit-and-undo.md](audit-and-undo.md), no double-record).
- [ ] Tests: resolution ordering, cycle detection, interpolation/quoting, multi/All
      expansion, dependency-driven invalidation, injection.

## Out of scope (hand off)

- Time range and `$__from`/`$__to`/`$__interval` → [time-range-and-refresh.md](time-range-and-refresh.md).
- `context` as a variable source (nav/url/tag) → [page-context-and-nav.md](page-context-and-nav.md).
- Repeat-by-variable *rendering* → a later boards/reflow concern; this exposes the list only.
