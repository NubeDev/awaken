# Good stuff to copy from lmnr

[lmnr](https://github.com/lmnr-ai/lmnr) is Apache-2.0, so copy anything useful.
Clone lives at `/tmp/lmnr` (reference only, never committed). Its `ui/` stack is
basically ours — React 19, `@xyflow/react` 12, recharts 3, `@tanstack/react-table`
8, Radix, Tailwind 4, zustand — so frontend components drop in. Keep the
Apache-2.0 header on anything copied.

## Dashboard SQL builder + charts

`lmnr/frontend/components/dashboards/` and `components/chart-builder/`

A dashboard chart is `{ query: SQL, settings: { config, layout: {x,y,w,h} } }` laid
out on `react-grid-layout`, each chart running its own query. Maps straight onto
our DataFusion `/query`.

- `chart-builder/charts/` — recharts wrappers: `line-chart.tsx`, `bar-chart.tsx`,
  `horizontal-bar-chart.tsx`, `table-chart.tsx` + `ChartConfig` type. This is a
  ready widget registry.
- `dashboards/grid-layout.tsx`, `chart.tsx`, `chart-header.tsx`, `types.ts` — the
  grid + per-chart query wiring.
- `dashboards/editor/` Builder + fields (`ChartTypeField`, `DimensionsField`,
  `MetricsField`, `FiltersField`, `OrderByField`, `LimitField`) — visual query
  builder that emits SQL for non-SQL authors.
- `components/sql/sql-editor.tsx` + `parameters-panel.tsx` — raw SQL editor with
  bound parameters.

## Span / run viewer

`lmnr/frontend/components/traces/trace-view/` and `span-view/`

A waterfall of timed bars + a detail panel per step. Good fit for viewing awaken
agent runs (diagnose → tool calls → LLM calls → gated write).

- `trace-view/` — waterfall layout, panel resize, timeline bars, detail panel.
- `span-view/messages.tsx`, `span-content.tsx` — render messages / tool I/O in a
  detail panel.

## Annotation / dataset UI

`lmnr/frontend/components/evaluation/` and `components/dataset/`

Labeling + dataset building. Useful later for labeling spark findings and agent
proposals ("was this finding correct?").

## Skip

Their backend (`app-server`, `query-engine`, `pii-redactor`), ClickHouse, the OTel
ingest, and the Next.js app shell (`app/`, server actions, `next-auth`). Take leaf
components, not the framework.
