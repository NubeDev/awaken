# Rubix Fleet/Dashboard Build — Workstream Queue

The unattended build queue for the four `docs/design/` fleet/dashboard scope docs. Driven by
[_ORCHESTRATION.md](./_ORCHESTRATION.md). Each row is a workstream (WS) with a spec doc in this
directory. Status legend: ⬜ pending · 🔵 in-progress · ✅ done · ⛔ blocked (see TODOs.md).

Branch: **`rubix-gaps`**. Design sources:
[variables-and-templating.md](../../design/variables-and-templating.md),
[time-range-and-refresh.md](../../design/time-range-and-refresh.md),
[page-context-and-nav.md](../../design/page-context-and-nav.md),
[audit-and-undo.md](../../design/audit-and-undo.md).

Queue order is dependency order — earlier rows ship contracts later rows build on. The four docs'
own "Out of scope (hand off)" sections encode this order: variables → time → context/nav, with the
audit ledger as a parallel substrate the others wire into.

| # | Workstream | Doc | Status | Started | Finished | Commit |
| --- | --- | --- | --- | --- | --- | --- |
| WS-01 | SQL interpolation engine in `rubix-query` + both query paths accept `variables` (injection boundary) | variables | ✅ | 2026-06-13T13:13:46Z | 2026-06-13T14:05:00Z | 4fe4e5b8 |
| WS-02 | Dashboard `variables` model + DTO/OpenAPI/TS + resolution/cascading + variable bar/editor + `?var-*` URL | variables | ⬜ | | | |
| WS-03 | Time macros (`$__from`/`$__to`/`$__timeFilter`/`$__timeGroup`/`$__interval`) in engine + query DTOs + frozen-`now` + `point_history` wiring | time-range | ⬜ | | | |
| WS-04 | Time store + relative resolver + TimeRangePicker + auto-refresh loop + `?from/to/refresh` URL + drag-zoom (UI) | time-range | ⬜ | | | |
| WS-05 | Entity-tag store/routes/authz + `nav_nodes` table + nav CRUD/reorder/reparent + `nav_node` grant kind + default-tree seed | page-context | ⬜ | | | |
| WS-06 | `context` VariableKind full-stack + `PageContext` assembly/precedence + `varRevision`/resolution wiring + nested sidebar + Navigation builder (UI) | page-context | ⬜ | | | |
| WS-07 | `changes` + `undo_cursors` tables + `ChangeRecorder` + `Reversible` registry + cascade grouping + coverage guard | audit-undo | ⬜ | | | |
| WS-08 | Record-on-mutate per kind (agent actor + secret redaction) + `GET /audit` read routes + `POST /undo`/`/redo` | audit-undo | ⬜ | | | |
| WS-09 | Undo/redo shortcuts + toast-with-undo + query invalidation + per-resource History tab + admin Audit screen (UI) | audit-undo | ⬜ | | | |

## Dependency notes
- **WS-01** is the foundation: the server-side injection-safe interpolation engine lives in the
  existing `rubix-query` crate (which today has providers/rollup/sql but **no variable binder**).
  WS-02, WS-03, and WS-06 all lower SQL through it. Nothing parameterised works until WS-01 lands.
- **WS-02** ships the dashboard variable model (stored in the dashboard JSON, not a table) and the
  UI; it introduces rubix's **first** URL query-param sync (`?var-*`), which WS-04 (time) and WS-06
  (context bare params) then share. WS-02 depends on WS-01's engine.
- **WS-03** extends WS-01's engine with the time macros and the query DTOs' `time_range`/
  `interval_secs`; **WS-04** is the UI half (store, picker, refresh, zoom) and reuses WS-02's URL
  mechanism. WS-03 must land before WS-04 can wire selections into queries.
- **WS-05** is the page-context/nav **backend**: the entity-tag store (tags become behaviour-
  affecting), the `nav_nodes` table, nav CRUD, and registering `nav_node` as a grantable kind in the
  existing grant model (`api/grants/`). **WS-06** is the **frontend + the `context` variable source**;
  it depends on WS-02 (variable engine/UI), WS-04 (the `varRevision` re-query path it folds context
  into), and WS-05 (nav + tag backends).
- **WS-07** lands the audit/undo **substrate** (the one append-only `changes` ledger + the
  `Reversible` registry + the coverage guard) independent of the other three docs. **WS-08** wires
  the `record` calls into every mutation handler (incl. `Actor::Agent` and secret redaction) and the
  audit-read + undo/redo HTTP surface. **WS-09** is the audit/undo UI. WS-08 depends on WS-07; WS-09
  on WS-08. The nav/tag/variable WSs note in their specs that their mutations get recorded by
  WS-08's recorder (no double-record — variable/context edits ride the dashboard snapshot).

## Loop log
<!-- The loop appends one line per wake here: <utc> <action> (spawned WS-xx / gated WS-xx ✅ / blocked WS-xx ⛔ / idle). -->
- (pending first wake)
- 2026-06-13T13:13:46Z spawned WS-01 (SQL interpolation engine, backend)
- 2026-06-13T14:05:00Z gated WS-01 ✅ (engine + both query paths accept `variables`, injection-safe)
