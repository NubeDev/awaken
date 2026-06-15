// One-click chart presets for boards — the Rubix analogue of Laminar's
// `chart-presets.ts` (§2). Laminar's are ClickHouse/trace-schema specific; ours
// target the canonical query surface (record / audit / trace_summary) and reach
// into `content` JSON with json_get. Time-series presets are parameterised on
// `{{start_time}}` / `{{end_time}}` / `{{interval_unit}}` so the board time
// range scopes them (see board-params.ts). Picking a preset creates a
// `kind:"chart"` record (charts.ts) and places it on the board.

import { ChartType, type ChartConfig } from '../chart-builder/types'

export type PresetGroup = 'Records' | 'Audit' | 'Traces'

export interface ChartPreset {
  name: string
  group: PresetGroup
  sql: string
  config: ChartConfig
}

export const CHART_PRESETS: ChartPreset[] = [
  {
    name: 'Records over time',
    group: 'Records',
    sql: `SELECT date_trunc('{{interval_unit}}', created) AS time, count(*) AS n
FROM record
WHERE created >= CAST({{start_time}} AS TIMESTAMP) AND created <= CAST({{end_time}} AS TIMESTAMP)
GROUP BY time ORDER BY time`,
    config: { type: ChartType.LineChart, x: 'time', y: 'n', displayMode: 'total' },
  },
  {
    name: 'Records by kind',
    group: 'Records',
    sql: `SELECT json_get(json_get(content, 'content'), 'kind') AS kind, count(*) AS n
FROM record GROUP BY kind ORDER BY n DESC`,
    config: { type: ChartType.HorizontalBarChart, x: 'n', y: 'kind', displayMode: 'total' },
  },
  {
    name: 'Records by namespace',
    group: 'Records',
    sql: `SELECT namespace, count(*) AS n FROM record GROUP BY namespace ORDER BY n DESC`,
    config: { type: ChartType.BarChart, x: 'namespace', y: 'n', displayMode: 'total' },
  },
  {
    name: 'Audit volume over time',
    group: 'Audit',
    sql: `SELECT date_trunc('{{interval_unit}}', created) AS time, count(*) AS n
FROM audit
WHERE created >= CAST({{start_time}} AS TIMESTAMP) AND created <= CAST({{end_time}} AS TIMESTAMP)
GROUP BY time ORDER BY time`,
    config: { type: ChartType.LineChart, x: 'time', y: 'n', displayMode: 'total' },
  },
  {
    name: 'Audit by action',
    group: 'Audit',
    sql: `SELECT json_get(content, 'action') AS action, count(*) AS n
FROM audit GROUP BY action ORDER BY n DESC`,
    config: { type: ChartType.BarChart, x: 'action', y: 'n', displayMode: 'total' },
  },
  {
    name: 'Trace status',
    group: 'Traces',
    sql: `SELECT json_get(content, 'status') AS status, count(*) AS n
FROM trace_summary GROUP BY status ORDER BY n DESC`,
    config: { type: ChartType.BarChart, x: 'status', y: 'n', displayMode: 'total' },
  },
  {
    name: 'Tokens per trace',
    group: 'Traces',
    sql: `SELECT json_get(content, 'trace_id') AS trace, CAST(json_get(content, 'total_tokens') AS BIGINT) AS tokens
FROM trace_summary ORDER BY tokens DESC`,
    config: { type: ChartType.HorizontalBarChart, x: 'tokens', y: 'trace', displayMode: 'none' },
  },
]
