// Admin · Query console — run a read-only query over POST /query (DataFusion),
// render the rows as a table or a chart (with drag-zoom + click-to-drill), and
// save the query (+ its chart config) as a `kind:"query"` record (§1a/§2/§3,
// LAMINAR-BORROW.md). A developer's ad-hoc window into the data; gated
// server-side on external-query. Renders whatever columns the result carries.

import { getRouteApi } from '@tanstack/react-router'
import { useMemo, useState } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { LineChart as LineChartIcon, Play, Save, Sparkles, TerminalSquare, Trash2, ZoomOut } from 'lucide-react'
import { useApi } from '../../api/ConnectionContext'
import { runQuery, type QueryResponse } from '../../api/query'
import { createChart } from '../../api/charts'
import {
  createSavedQuery,
  deleteSavedQuery,
  listSavedQueries,
  updateSavedQuery,
  type SavedQuery,
} from '../../api/savedQueries'
import { AdminLayout } from '../../components/admin/AdminLayout'
import { ErrorView } from '../../components/ui/StateView'
import { Button } from '../../components/ui/button'
import { Input } from '../../components/ui/input'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '../../components/ui/select'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '../../components/ui/tabs'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '../../components/ui/table'
import { SqlEditor } from '../../components/sql/SqlEditor'
import { ChartRendererCore } from '../../components/chart-builder/charts'
import { useChartZoom } from '../../components/chart-builder/charts/useChartZoom'
import { transformDataToColumns, type ColumnInfo, type DataRow } from '../../components/chart-builder/utils'
import { ChartType, type ChartConfig, type DisplayMode } from '../../components/chart-builder/types'

const route = getRouteApi('/t/$tenant/admin/query')

// The query surface exposes structural columns (id, namespace, created, updated)
// plus `content` as a JSON *string* — there are no JSON UDFs registered, so
// `content.kind`-style field access fails. These presets use only structural
// columns, so each runs as-is and charts directly.
const STARTER =
  "SELECT date_trunc('day', created) AS day, count(*) AS n FROM record GROUP BY day ORDER BY day"

const NONE = '__none__'
const NEW = '__new__'

interface Preset {
  name: string
  sql: string
  chart: ChartConfig
}

const PRESETS: Preset[] = [
  { name: 'Records per day', sql: STARTER, chart: { type: ChartType.LineChart, x: 'day', y: 'n' } },
  {
    name: 'Records per namespace',
    sql: 'SELECT namespace, count(*) AS n FROM record GROUP BY namespace ORDER BY n DESC',
    chart: { type: ChartType.BarChart, x: 'namespace', y: 'n' },
  },
  {
    name: 'Audit volume per day',
    sql: "SELECT date_trunc('day', created) AS day, count(*) AS n FROM audit GROUP BY day ORDER BY day",
    chart: { type: ChartType.LineChart, x: 'day', y: 'n' },
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

  const saved = useQuery({
    queryKey: ['saved-queries', tenant],
    queryFn: () => listSavedQueries(api),
  })

  const query = useMutation<QueryResponse, Error, string>({
    mutationFn: (text) => runQuery(api, text),
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

  // Rows shown in the chart, narrowed to the active drag-zoom window (if any).
  const chartRows = useMemo(() => zoom.apply(rows, chart.x), [zoom, rows, chart.x])

  function run() {
    if (sql.trim()) query.mutate(sql)
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
    query.mutate(p.sql)
  }

  return (
    <AdminLayout active="query">
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

        <SqlEditor value={sql} onChange={setSql} onRun={run} />

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
              {drillRow && <DrillPanel row={drillRow} onClose={() => setDrillRow(null)} />}
            </TabsContent>
          </Tabs>
        )}
      </div>
    </AdminLayout>
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

// The chart builder fields on the committed spine: type, x/y, breakdown
// (multi-series split), and display mode (headline total/average). The no-SQL
// structured builder (§1c) stays deferred.
function ChartConfigBar({
  columns,
  config,
  onChange,
}: {
  columns: ColumnInfo[]
  config: ChartConfig
  onChange: (config: ChartConfig) => void
}) {
  const set = (patch: Partial<ChartConfig>) => onChange({ ...config, ...patch } as ChartConfig)
  const isAxis = config.type !== ChartType.Table
  return (
    <div className="flex flex-wrap items-center gap-2">
      <Select value={config.type ?? ChartType.LineChart} onValueChange={(v) => set({ type: v as ChartType })}>
        <SelectTrigger className="w-[140px]">
          <SelectValue placeholder="Chart type" />
        </SelectTrigger>
        <SelectContent>
          <SelectItem value={ChartType.LineChart}>Line</SelectItem>
          <SelectItem value={ChartType.BarChart}>Bar</SelectItem>
          <SelectItem value={ChartType.HorizontalBarChart}>Horizontal bar</SelectItem>
          <SelectItem value={ChartType.Table}>Table</SelectItem>
        </SelectContent>
      </Select>
      {isAxis && (
        <>
          <ColumnPicker label="X" value={config.x} columns={columns} onChange={(x) => set({ x })} />
          <ColumnPicker label="Y" value={config.y} columns={columns} onChange={(y) => set({ y })} />
          <ColumnPicker
            label="Breakdown"
            value={config.breakdown}
            columns={columns}
            allowNone
            onChange={(b) => set({ breakdown: b === NONE ? undefined : b })}
          />
          <Select
            value={config.displayMode ?? 'none'}
            onValueChange={(v) => set({ displayMode: v as DisplayMode })}
          >
            <SelectTrigger className="w-[140px]">
              <SelectValue placeholder="Display" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="none">No headline</SelectItem>
              <SelectItem value="total">Total</SelectItem>
              <SelectItem value="average">Average</SelectItem>
            </SelectContent>
          </Select>
        </>
      )}
    </div>
  )
}

function ColumnPicker({
  label,
  value,
  columns,
  allowNone,
  onChange,
}: {
  label: string
  value?: string
  columns: { name: string }[]
  allowNone?: boolean
  onChange: (value: string) => void
}) {
  return (
    <Select value={value ?? (allowNone ? NONE : '')} onValueChange={onChange}>
      <SelectTrigger className="w-[150px]">
        <SelectValue placeholder={label} />
      </SelectTrigger>
      <SelectContent>
        {allowNone && <SelectItem value={NONE}>No {label.toLowerCase()}</SelectItem>}
        {columns.map((c) => (
          <SelectItem key={c.name} value={c.name}>
            {c.name}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  )
}

function renderCell(value: unknown): string {
  if (value === null || value === undefined) return '—'
  if (typeof value === 'object') return JSON.stringify(value)
  return String(value)
}
