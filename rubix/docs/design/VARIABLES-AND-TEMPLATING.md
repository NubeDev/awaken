# VARIABLES-AND-TEMPLATING — one board, parameterised across a fleet

Feature scope for **dashboard variables**: one board, authored once, served across a
whole fleet. A variable bar lets a user pick values (`$site`, `$equip`, `$building`);
each widget re-queries against the selection; the values deep-link via URL. This is
the feature that turns *N near-identical boards* into *one board + a dropdown*,
directly serving the building/energy/HVAC use case rubix targets.

It re-grounds the nexus WS-02 / old-rubix `variables-and-templating.md` design in the
**records-backed** rubix here (`docs/design/DASHBOARDS-SCOPE.md`): a board is a
`kind:"board"` record, a chart a `kind:"chart"` record, and queries run through the
unified DataFusion surface (`POST /query`, `POST /query/batch`). The crucial rubix
fact: **the unified surface has no SQL bind layer** — `ctx.sql(sql)` takes a string
only — so this scope owns *both* the variable model *and* the server-side
interpolation engine, built injection-safe.

> **Status — backend shipped.** The server-side engine and the wire contract (§2,
> §6) are implemented and tested (`crates/rubix-query/src/template/`,
> `crates/rubix-server/tests/http/query/variables_test.rs`). The board-side model,
> bar, editor, resolution, and URL state (§1, §3, §4, §5, §7) are the **UI scope**
> and consume the contract below.

## Problem / current state

- A chart's query is **static SQL** frozen at author time. The chart record is
  `{ kind:"chart", name, sql, config }` (`ui/src/api/charts.ts`); `sql` is a raw
  string.
- `POST /query` / `POST /query/batch` take `{ sql, time?, quantities?, transforms? }`
  (`crates/rubix-server/src/dto/query.rs`). The only pre-execution rewrite was the
  **time macros** (`$__timeFilter`/`$__timeBucket`/`$__interval`,
  `crates/rubix-query/src/time/rewrite.rs`) — there was **no variable
  interpolation** and no bind path.
- The board has one board-wide time control (`ui/src/components/dashboards/board-params.ts`)
  but **no variable concept** — no `$site` a dropdown can drive.

Consequence: serving M sites meant M boards. There was no `WHERE site = $site`.

## Scope

### 1. Variable model (UI — board JSON)

A `Variable` lives in the **board record's JSON** (`content.variables`), so it travels
with the board on export/import — no separate relational table (the board is the unit,
matching how panels already ride `content.panels`). The board content grows to
`{ kind:"board", name, panels, variables? }`.

```
Variable {
  name:        string         // referenced as $name / ${name}
  label?:      string
  kind:        VariableKind
  config:      VariableConfig  // per-kind (tagged)
  current:     Scalar | Scalar[]   // selection (a Scalar is string | number | bool)
  multi:       bool
  include_all: bool
  hidden:      bool
}
```

**Variable kinds** (closed set; adding one is a UI change + a resolver arm):

- `constant` — fixed value, usually hidden.
- `custom` — a static option list.
- `query` — options from running SQL (one column → options); re-runs `POST /query`.
- `site` — options are the namespace's sites (`kind:"site"` records). The headline
  rubix kind: the natural fleet axis is the site.
- `datasource` — options are the namespace's datasources of a kind.
- `textbox` — free text.
- built-ins (read-only): `$__from`/`$__to` (from the board time scope), `$__interval`
  (the resolved grain). The `context` kind (nav/url/tag sources) is added by
  [PAGE-CONTEXT-AND-NAV.md](./PAGE-CONTEXT-AND-NAV.md).

### 2. Server-side interpolation engine — the injection boundary (SHIPPED)

`crates/rubix-query/src/template/` lowers variable references in SQL into **escaped
SQL literals**, server-side, **before** the read-only guard. Because there is no bind
path, *the engine owns the quoting* — and that is the entire security story:

- A `Text` value → a single-quoted literal with every `'` doubled. DataFusion's
  dialect has no backslash escape, so doubling `'` is complete: `'); DROP TABLE
  record; --` lowers to the literal `'''); DROP TABLE record; --'` and cannot break
  out of its string.
- A `Num`/`Bool` value → a closed character set (digits/sign/`.`/`e`, or
  `TRUE`/`FALSE`) lowered bare; it can carry no metacharacter.

The lowering runs after the time macros (which own `$__time*`) and before
`rubix_query::ensure_read_only` (run by `span`), so the guard always vets the final,
fully-resolved single statement — a value can never smuggle a second one past it.
**Two layers**: escaped-literal lowering, then the statement guard.

**Reference forms** (`expand_variables(sql, &[Variable])`):

| Form | Lowers to | Use |
|------|-----------|-----|
| `$name` / `${name}` | first value as one literal (`NULL` if empty) | `WHERE site = $site` |
| `${name:csv}` | every value as a typed literal, comma-joined (`NULL` if empty) | `WHERE site IN (${site:csv})` |
| `${name:singlequote}` | every value force-quoted as a string, comma-joined | string `IN` lists |
| `$__sqlIn(name)` | parenthesised `(v1, v2, …)`; empty → `(NULL)` (matches none) | `WHERE site IN $__sqlIn(site)` |

**The author never writes their own quotes around a variable** (`= $site`, not
`= '$site'`) — the engine emits a complete literal. An unsupplied **bare** `$name` is
left as a literal `$` (so `$5` or a stray `$` survives); an unsupplied **explicit**
`${…}` / `$__sqlIn(…)` is a rejection (it declared an intent). A leading-underscore
token (`$__interval`) is never mistaken for a variable.

### 3–5, 7. Variable bar, editor, resolution, URL state (UI)

- **Bar** (`ui/src/features/variables/**`, new): each visible variable as a single-
  or multi-select (or textbox), with an "All" option when `include_all`; mounted above
  the board canvas.
- **Editor**: in the board settings dialog — add/edit/reorder/delete, choose kind,
  author the option query, preview resolved options, set multi/include-all/hidden.
- **Resolution** (`resolve.ts`, new): order built-ins → constants/custom →
  site/datasource → `query` (topological by dependency). A `query` variable's SQL may
  reference another variable (`WHERE site = '$site'`) — build a dependency order and
  **reject cycles**. Cache option lists per `(name, parent-values, timeRange)`.
- **URL state**: `?var-site=hq&var-site=tower` (repeatable for multi); restore on load;
  shareable. Keep the `var-` prefix reserved for explicit variable state (bare params
  belong to [PAGE-CONTEXT-AND-NAV.md](./PAGE-CONTEXT-AND-NAV.md)).

### 6. Re-query on change — the wire contract (SHIPPED server side)

`POST /query` and each `POST /query/batch` item accept an optional `variables` array:

```jsonc
{
  "sql": "SELECT count(*) FROM record WHERE json_get(json_get(content,'content'),'site') = $site",
  "variables": [ { "name": "site", "value": "hq" } ]   // value: scalar | scalar[]
}
```

`value` is a JSON scalar (single-select) or an array of scalars (multi-select); a
null is an empty selection; an object / nested array is rejected (`400`) — only
scalars reach the SQL (`QueryVariableDto::into_variable`).

UI side: fold the resolved variable values into a `varRevision` (a hash) and add it to
each templated widget's **data** query key, so a selection change re-fetches exactly
the widgets whose SQL references a variable; a widget with no reference is unaffected
(back-compat). One revision, bumped on any selection change.

## What to prove

1. A `query` variable's dropdown is populated from live SQL. *(UI)*
2. A widget using `site = $site` (or `$__sqlIn(site)`) re-queries when the selection
   changes; an unrelated widget does not. *(UI key)*
3. Multi-select + "All" produce a correct, safely-quoted `IN (...)`. *(engine — done:
   `a_multi_select_expands_a_safe_in_list`)*
4. Cascading: changing `$site` re-resolves `$equip`; a cycle is rejected. *(UI)*
5. Values deep-link via `?var-…` and restore on reload. *(UI)*
6. Injection: a value containing `'); DROP …` binds as a literal, never executes.
   *(engine — done: `an_injection_payload_binds_as_a_literal_and_runs_safely`)*

## Acceptance criteria

- [x] Server interpolation engine: `$var`, `${var}`, `${var:csv}`,
      `${var:singlequote}`, `$__sqlIn(var)`; every value an escaped literal
      (injection-safe); both query paths accept `variables`; registered in OpenAPI.
- [x] Engine + route tests: lowering/quoting, multi/empty expansion, prefix
      collisions, unknown/unclosed rejection, end-to-end injection neutralised.
- [x] `variables` field on the board record JSON + TS mirror (`ui/src/api/boards.ts`
      `BoardVariable`); travels with the board (carried through layout writes).
- [x] Variable kinds: constant, custom, query, site, textbox (datasource + built-ins
      pending). Resolved in `ui/src/components/dashboards/useBoardVariables.ts`.
- [x] Variable **bar** mounted on the board (`VariableBar.tsx`), single/multi/All +
      textbox. Editor (author variables) + option preview still pending.
- [~] Resolution order + one-level cascade (a `query` variable reads its parents'
      selections); full topological cascade + cycle detection pending.
- [x] `varRevision` folded into the `board-batch` query key (`DashboardGrid.tsx`).
- [x] URL `?var-*` round-trips (router `validateBoardSearch` + `setVar`).

## Design notes

- **Variables live in the board JSON, not a table.** Keeps export/import
  self-contained and lets a board edit stay one audited gate write (boards already
  persist their whole `content` per change — `ui/src/api/boards.ts`).
- **Site is the first-class fleet axis.** Every domain entity carries a `site` in its
  content; a `site` kind that resolves to that is the common path and the editor
  default.
- **All / multi:** explicit expansion first (`All` → full `IN` list) — predictable and
  pushdown-friendly — before any wildcard token.
- **Quoting is mandatory and server-side.** The board may send any value; the engine,
  not the author, decides the literal. This is why authors must not wrap a variable in
  quotes, and why the guard remains the second layer.

## Out of scope (hand off)

- Time range & `$__from`/`$__to`/`$__interval` → the board time scope already shipped
  (`DASHBOARDS-SCOPE.md` §5).
- `context` as a variable source (nav / url / tag) →
  [PAGE-CONTEXT-AND-NAV.md](./PAGE-CONTEXT-AND-NAV.md) (adds the `context` kind + the
  `$__` context tokens; reuses this engine unchanged).
- Repeat-by-variable *rendering* → later (this exposes the resolved value list only).
