// Admin · Query console — run a read-only query over POST /query (DataFusion),
// render the rows as a table or a chart (with drag-zoom + click-to-drill), and
// save the query (+ its chart config) as a `kind:"query"` record (§1a/§2/§3,
// LAMINAR-BORROW.md). A developer's ad-hoc window into the data; gated
// server-side on external-query. Renders whatever columns the result carries.

import { getRouteApi } from '@tanstack/react-router'
import { useMemo, useState } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { LineChart as LineChartIcon, Play, Save, Sparkles, TerminalSquare, Trash2, ZoomOut } from 'lucide-react'
import { useApi } from '../../../api/ConnectionContext'
import { runQuery, type QueryResponse } from '../../../api/query'
import { createChart } from '../../../api/charts'
import {
  createSavedQuery,
  deleteSavedQuery,
  listSavedQueries,
  updateSavedQuery,
  type SavedQuery,
} from '../../../api/savedQueries'
import { usePageHeader } from '../../../components/shell/page-header'
import { ErrorView } from '../../../components/ui/StateView'
import { Button } from '../../../components/ui/button'
import { Input } from '../../../components/ui/input'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '../../../components/ui/select'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '../../../components/ui/tabs'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '../../../components/ui/table'
import { SqlEditor } from '../../../components/sql/SqlEditor'
import { ParametersPanel } from '../../../components/sql/ParametersPanel'
import { applyParameters, useSqlEditorStore } from '../../../components/sql/sql-editor-store'
import { ChartRendererCore } from '../../../components/chart-builder/charts'
import { useChartZoom } from '../../../components/chart-builder/charts/useChartZoom'
import { transformDataToColumns, type DataRow } from '../../../components/chart-builder/utils'
import { ChartType, type ChartConfig } from '../../../components/chart-builder/types'
import { TransformEditor } from '../../../components/chart-builder/TransformEditor'
import { FieldConfigEditor } from '../../../components/chart-builder/FieldConfigEditor'
import { ChartConfigBar } from '../../../components/chart-builder/ChartConfigBar'
import type { FieldConfig } from '../../../components/chart-builder/field-config'
import { applyCosmeticTransforms, splitTransforms, type Transform } from '../../../components/chart-builder/transforms'

const route = getRouteApi('/t/$tenant/admin/query')

// The query surface exposes structural columns (id, namespace, created, updated)
// plus `content` as JSON text. The json_get(json, key) UDF reaches into it —
// composably, so json_get(json_get(content,'content'),'kind') descends to the
// document payload's `kind` (see the "Records by kind" preset). The starter uses
// only structural columns so it runs on any backend; json_get needs the rebuilt
// rubix-query.
const STARTER =
  "SELECT date_trunc('day', created) AS day, count(*) AS n FROM record GROUP BY day ORDER BY day"

const BY_KIND =
  "SELECT json_get(json_get(content, 'content'), 'kind') AS kind, count(*) AS n FROM record GROUP BY kind ORDER BY n DESC"

const NEW = '__new__'

interface Preset {
  name: string
  sql: string
  chart: ChartConfig
}

const PRESETS: Preset[] = [
  {
    name: 'Records by kind',
    sql: BY_KIND,
    chart: { type: ChartType.HorizontalBarChart, x: 'n', y: 'kind' },
  },
  {
    name: 'Records per day',
    sql: STARTER,
    chart: { type: ChartType.LineChart, x: 'day', y: 'n' },
  },
  {
    name: 'Records per namespace',
    sql: 'SELECT namespace, count(*) AS n FROM record GROUP BY namespace ORDER BY n DESC',
    chart: { type: ChartType.BarChart, x: 'namespace', y: 'n' },
  },
  {
    name: 'Audit by action',
    sql: "SELECT json_get(content, 'action') AS action, count(*) AS n FROM audit GROUP BY action ORDER BY n DESC",
    chart: { type: ChartType.BarChart, x: 'action', y: 'n' },
  },
  {
    name: 'Evaluations by group',
    sql: "SELECT json_get(json_get(content, 'content'), 'group_id') AS grp, count(*) AS n FROM record WHERE json_get(json_get(content, 'content'), 'group_id') IS NOT NULL GROUP BY grp ORDER BY n DESC",
    chart: { type: ChartType.BarChart, x: 'grp', y: 'n' },
  },
  {
    name: 'Trace status',
    sql: "SELECT json_get(content, 'status') AS status, count(*) AS n FROM trace_summary GROUP BY status ORDER BY n DESC",
    chart: { type: ChartType.BarChart, x: 'status', y: 'n' },
  },
  {
    name: 'Tokens per trace',
    sql: "SELECT json_get(content, 'trace_id') AS trace, CAST(json_get(content, 'total_tokens') AS BIGINT) AS tokens FROM trace_summary ORDER BY tokens DESC",
    chart: { type: ChartType.HorizontalBarChart, x: 'tokens', y: 'trace' },
  },
]

export function AdminQuery() {
  const { tenant } = route.useParams()
  const api = useApi(tenant)
  const qc = useQueryClient()

  const [sql, setSql] = useState(STARTER)
  const [name, setName] = useState('')
  const [selectedId, setSelectedId] = useState<string>(NEW)
  const [chart, setChart] = useState<ChartConfig>({ type: ChartType.LineChart })
  const [drillRow, setDrillRow] = useState<Record<string, any> | null>(null)

  const zoom = useChartZoom()
  const { parameters, setParameterValue, getFormattedParameters } = useSqlEditorStore()

  // Resolve `{{start_time}}`/`{{end_time}}`/`{{interval_unit}}` placeholders to
  // the parameter panel's current values before a query runs. Saved queries keep
  // the raw, parameterised SQL — only the executed text is substituted.
  const resolve = (text: string) => applyParameters(text, getFormattedParameters())

  const saved = useQuery({
    queryKey: ['saved-queries', tenant],
    queryFn: () => listSavedQueries(api),
  })

  // The console preview honours the chart's transforms (§1): aggregate ops go to
  // the backend, cosmetic ops are applied to the previewed rows below.
  const query = useMutation<QueryResponse, Error, string>({
    mutationFn: (text) =>
      runQuery(api, text, { transforms: splitTransforms(chart.transforms).aggregate }),
    onSuccess: () => {
      zoom.reset()
      setDrillRow(null)
    },
  })

  const persist = useMutation<SavedQuery, Error, void>({
    mutationFn: async () => {
      const input = { name: name.trim() || 'Untitled query', sql, chart }
      return selectedId === NEW
        ? createSavedQuery(api, input)
        : updateSavedQuery(api, selectedId, input)
    },
    onSuccess: (q) => {
      setSelectedId(q.id)
      void qc.invalidateQueries({ queryKey: ['saved-queries', tenant] })
    },
  })

  // Persist the current query + chart config as a kind:"chart" record, so it can
  // be pinned to a dashboard board (§2).
  const saveChart = useMutation({
    mutationFn: () =>
      createChart(api, { name: name.trim() || 'Untitled chart', sql, config: chart }),
    onSuccess: () => void qc.invalidateQueries({ queryKey: ['charts', tenant] }),
  })

  const remove = useMutation<void, Error, string>({
    mutationFn: (id) => deleteSavedQuery(api, id),
    onSuccess: () => {
      setSelectedId(NEW)
      void qc.invalidateQueries({ queryKey: ['saved-queries', tenant] })
    },
  })

  const rows = query.data?.rows ?? []

  const columns = useMemo(() => {
    const set = new Set<string>()
    for (const row of rows) for (const key of Object.keys(row)) set.add(key)
    return [...set]
  }, [rows])

  const chartColumns = useMemo(() => transformDataToColumns(rows as DataRow[]), [rows])

  // Rows shown in the chart: cosmetic transforms applied client-side (§1, the
  // aggregate tier already ran server-side), then narrowed to the drag-zoom window.
  const transformedRows = useMemo(
    () => applyCosmeticTransforms(rows, splitTransforms(chart.transforms).cosmetic),
    [rows, chart.transforms],
  )
  const chartRows = useMemo(
    () => zoom.apply(transformedRows, chart.x),
    [zoom, transformedRows, chart.x],
  )

  function run() {
    if (sql.trim()) query.mutate(resolve(sql))
  }

  function load(id: string) {
    setSelectedId(id)
    if (id === NEW) {
      setName('')
      setSql(STARTER)
      setChart({ type: ChartType.LineChart })
      return
    }
    const q = saved.data?.find((s) => s.id === id)
    if (q) {
      setName(q.name)
      setSql(q.sql)
      setChart(q.chart ?? { type: ChartType.LineChart })
    }
  }

  function applyPreset(presetName: string) {
    const p = PRESETS.find((x) => x.name === presetName)
    if (!p) return
    setName(p.name)
    setSql(p.sql)
    setChart(p.chart)
    setSelectedId(NEW)
    query.mutate(resolve(p.sql))
  }

  usePageHeader({ crumbs: ['Admin', 'Query'] })

  return (
    <div className="px-6 py-6">
      <div className="mx-auto max-w-[1100px]">
        <div className="mb-5 flex items-center gap-3">
          <div className="grid size-11 place-items-center rounded-xl border border-border bg-card">
            <TerminalSquare size={20} className="text-muted-foreground" />
          </div>
          <div>
            <h1 className="text-[22px] font-semibold tracking-tight">Query</h1>
            <div className="text-[13px] text-muted-foreground">
              Run a read-only query over the data plane, chart it, and save it.
            </div>
          </div>
        </div>

        {/* Saved-query + preset bar. */}
        <div className="mb-3 flex flex-wrap items-center gap-2">
          <Select value={selectedId} onValueChange={load}>
            <SelectTrigger className="w-[200px]">
              <SelectValue placeholder="Saved queries" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value={NEW}>+ New query</SelectItem>
              {(saved.data ?? []).map((q) => (
                <SelectItem key={q.id} value={q.id}>
                  {q.name}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <Select value="" onValueChange={applyPreset}>
            <SelectTrigger className="w-[190px]">
              <span className="flex items-center gap-1.5 text-muted-foreground">
                <Sparkles size={14} /> <SelectValue placeholder="Presets" />
              </span>
            </SelectTrigger>
            <SelectContent>
              {PRESETS.map((p) => (
                <SelectItem key={p.name} value={p.name}>
                  {p.name}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <Input
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="Query name"
            className="w-[190px]"
          />
          <Button variant="outline" onClick={() => persist.mutate()} disabled={persist.isPending} className="gap-1.5">
            <Save size={15} /> {persist.isPending ? 'Saving…' : 'Save'}
          </Button>
          <Button
            variant="outline"
            onClick={() => saveChart.mutate()}
            disabled={saveChart.isPending}
            className="gap-1.5"
            title="Save as a chart you can pin to a dashboard"
          >
            <LineChartIcon size={15} /> {saveChart.isPending ? 'Saving…' : 'Save as chart'}
          </Button>
          {selectedId !== NEW && (
            <Button
              variant="ghost"
              onClick={() => remove.mutate(selectedId)}
              disabled={remove.isPending}
              className="gap-1.5 text-muted-foreground"
            >
              <Trash2 size={15} /> Delete
            </Button>
          )}
        </div>

        <Tabs defaultValue="query">
          <TabsList>
            <TabsTrigger value="query">Query</TabsTrigger>
            <TabsTrigger value="parameters">Parameters</TabsTrigger>
          </TabsList>
          <TabsContent value="query">
            <SqlEditor value={sql} onChange={setSql} onRun={run} />
            <p className="mt-1.5 text-xs text-muted-foreground">
              Reference parameters as <code className="mono">{'{{start_time}}'}</code> /{' '}
              <code className="mono">{'{{end_time}}'}</code> /{' '}
              <code className="mono">{'{{interval_unit}}'}</code> — set their values in the
              Parameters tab.
            </p>
          </TabsContent>
          <TabsContent value="parameters">
            <ParametersPanel parameters={parameters} onChange={setParameterValue} />
          </TabsContent>
        </Tabs>

        <div className="mt-3 flex items-center gap-3">
          <Button onClick={run} disabled={query.isPending} className="gap-1.5">
            <Play size={15} /> {query.isPending ? 'Running…' : 'Run'}
          </Button>
          <span className="text-xs text-muted-foreground">⌘/Ctrl + Enter</span>
          {query.data && (
            <span className="ml-auto text-xs text-muted-foreground">
              {rows.length} {rows.length === 1 ? 'row' : 'rows'}
            </span>
          )}
        </div>

        {query.error && (
          <div className="mt-5">
            <ErrorView error={query.error} />
          </div>
        )}

        {query.data && (
          <Tabs defaultValue="results" className="mt-5">
            <TabsList>
              <TabsTrigger value="results">Results</TabsTrigger>
              <TabsTrigger value="chart">Chart</TabsTrigger>
            </TabsList>

            <TabsContent value="results">
              <ResultsTable columns={columns} rows={rows} />
            </TabsContent>

            <TabsContent value="chart">
              <div className="flex flex-wrap items-center gap-2">
                <ChartConfigBar columns={chartColumns} config={chart} onChange={setChart} />
                <FieldConfigEditor
                  value={chart.fieldConfig}
                  onChange={(fieldConfig: FieldConfig | undefined) => setChart({ ...chart, fieldConfig })}
                />
                {zoom.zoomed && (
                  <Button variant="ghost" onClick={zoom.reset} className="gap-1.5 text-muted-foreground">
                    <ZoomOut size={15} /> Reset zoom
                  </Button>
                )}
              </div>
              <div className="mt-3 h-[360px] rounded-xl border border-border bg-card/40 p-4">
                {rows.length === 0 ? (
                  <div className="grid h-full place-items-center text-sm text-muted-foreground">No rows to chart.</div>
                ) : (
                  <ChartRendererCore
                    config={chart}
                    data={chartRows}
                    columns={chartColumns}
                    syncId="query-console"
                    drag={zoom.drag}
                    onBarClick={setDrillRow}
                  />
                )}
              </div>
              {chart.type === ChartType.LineChart || chart.type === ChartType.BarChart ? (
                <p className="mt-2 text-xs text-muted-foreground">Drag across the chart to zoom into a range.</p>
              ) : null}
              {/* Transform pipeline (§1): aggregate ops apply on the next Run;
                  cosmetic ops apply to the preview immediately. */}
              <div className="mt-4 rounded-xl border border-border bg-card/40 p-3">
                <TransformEditor
                  value={chart.transforms}
                  onChange={(transforms: Transform[]) => setChart({ ...chart, transforms })}
                />
              </div>
              {drillRow && <DrillPanel row={drillRow} onClose={() => setDrillRow(null)} />}
            </TabsContent>
          </Tabs>
        )}
      </div>
    </div>
  )
}

function ResultsTable({ columns, rows }: { columns: string[]; rows: Record<string, unknown>[] }) {
  if (rows.length === 0) {
    return (
      <div className="rounded-xl border border-border bg-card/40 px-4 py-6 text-center text-sm text-muted-foreground">
        No rows returned.
      </div>
    )
  }
  return (
    <div className="rounded-xl border border-border bg-card/40">
      <Table>
        <TableHeader>
          <TableRow>
            {columns.map((c) => (
              <TableHead key={c} className="mono">
                {c}
              </TableHead>
            ))}
          </TableRow>
        </TableHeader>
        <TableBody>
          {rows.map((row, i) => (
            <TableRow key={i}>
              {columns.map((c) => (
                <TableCell key={c} className="mono text-xs">
                  {renderCell(row[c])}
                </TableCell>
              ))}
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </div>
  )
}

// Click-to-drill target: shows the underlying row of a clicked bar/cell. The §3
// "click a point → open the underlying row/trace" behaviour, contained to the
// rows already on screen.
function DrillPanel({ row, onClose }: { row: Record<string, any>; onClose: () => void }) {
  return (
    <div className="mt-3 rounded-xl border border-border bg-card/60 p-4">
      <div className="mb-2 flex items-center justify-between">
        <span className="text-sm font-medium">Row detail</span>
        <Button variant="ghost" onClick={onClose} className="h-7 text-xs text-muted-foreground">
          Close
        </Button>
      </div>
      <pre className="overflow-auto rounded-md bg-bg/50 p-3 text-xs mono">{JSON.stringify(row, null, 2)}</pre>
    </div>
  )
}

function renderCell(value: unknown): string {
  if (value === null || value === undefined) return '—'
  if (typeof value === 'object') return JSON.stringify(value)
  return String(value)
}
