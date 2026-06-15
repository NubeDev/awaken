// Admin · Query console — run a read-only query over POST /query (DataFusion),
// render the rows as a table or a chart, and save the query (+ its chart config)
// as a `kind:"query"` record (§1a/§2, LAMINAR-BORROW.md). A developer's ad-hoc
// window into the data; gated server-side on the external-query capability.
// Renders whatever columns the result carries — no domain assumptions.

import { getRouteApi } from '@tanstack/react-router'
import { useMemo, useState } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Play, Save, TerminalSquare, Trash2 } from 'lucide-react'
import { useApi } from '../../api/ConnectionContext'
import { runQuery, type QueryResponse } from '../../api/query'
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
import { transformDataToColumns, type DataRow } from '../../components/chart-builder/utils'
import { ChartType, type ChartConfig } from '../../components/chart-builder/types'

const route = getRouteApi('/t/$tenant/admin/query')

const STARTER = 'SELECT content.kind AS kind, count() AS n FROM record GROUP BY kind'

const NEW = '__new__'

export function AdminQuery() {
  const { tenant } = route.useParams()
  const api = useApi(tenant)
  const qc = useQueryClient()

  const [sql, setSql] = useState(STARTER)
  const [name, setName] = useState('')
  const [selectedId, setSelectedId] = useState<string>(NEW)
  const [chart, setChart] = useState<ChartConfig>({ type: ChartType.LineChart })

  const saved = useQuery({
    queryKey: ['saved-queries', tenant],
    queryFn: () => listSavedQueries(api),
  })

  const query = useMutation<QueryResponse, Error, string>({
    mutationFn: (text) => runQuery(api, text),
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

  // Typed columns for the chart-config pickers (string/number/boolean inference).
  const chartColumns = useMemo(() => transformDataToColumns(rows as DataRow[]), [rows])

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

        {/* Saved-query bar: load an existing query or start a new one, name it, save it. */}
        <div className="mb-3 flex flex-wrap items-center gap-2">
          <Select value={selectedId} onValueChange={load}>
            <SelectTrigger className="w-[220px]">
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
          <Input
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="Query name"
            className="w-[220px]"
          />
          <Button variant="outline" onClick={() => persist.mutate()} disabled={persist.isPending} className="gap-1.5">
            <Save size={15} /> {persist.isPending ? 'Saving…' : 'Save'}
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
              <div className="rounded-xl border border-border bg-card/40">
                {rows.length === 0 ? (
                  <div className="px-4 py-6 text-center text-sm text-muted-foreground">No rows returned.</div>
                ) : (
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
                )}
              </div>
            </TabsContent>

            <TabsContent value="chart">
              <ChartConfigBar columns={chartColumns} config={chart} onChange={setChart} />
              <div className="mt-3 h-[360px] rounded-xl border border-border bg-card/40 p-4">
                {rows.length === 0 ? (
                  <div className="grid h-full place-items-center text-sm text-muted-foreground">No rows to chart.</div>
                ) : (
                  <ChartRendererCore config={chart} data={rows} columns={chartColumns} />
                )}
              </div>
            </TabsContent>
          </Tabs>
        )}
      </div>
    </AdminLayout>
  )
}

// The minimal chart picker: type + x/y columns. The full field set (breakdown,
// display mode, the no-SQL builder) is deferred (§1c); this covers the spine.
function ChartConfigBar({
  columns,
  config,
  onChange,
}: {
  columns: { name: string; type: string }[]
  config: ChartConfig
  onChange: (config: ChartConfig) => void
}) {
  const set = (patch: Partial<ChartConfig>) => onChange({ ...config, ...patch } as ChartConfig)
  return (
    <div className="flex flex-wrap items-center gap-2">
      <Select value={config.type ?? ChartType.LineChart} onValueChange={(v) => set({ type: v as ChartType })}>
        <SelectTrigger className="w-[150px]">
          <SelectValue placeholder="Chart type" />
        </SelectTrigger>
        <SelectContent>
          <SelectItem value={ChartType.LineChart}>Line</SelectItem>
          <SelectItem value={ChartType.BarChart}>Bar</SelectItem>
          <SelectItem value={ChartType.HorizontalBarChart}>Horizontal bar</SelectItem>
          <SelectItem value={ChartType.Table}>Table</SelectItem>
        </SelectContent>
      </Select>
      <ColumnPicker label="X" value={config.x} columns={columns} onChange={(x) => set({ x })} />
      <ColumnPicker label="Y" value={config.y} columns={columns} onChange={(y) => set({ y })} />
    </div>
  )
}

function ColumnPicker({
  label,
  value,
  columns,
  onChange,
}: {
  label: string
  value?: string
  columns: { name: string }[]
  onChange: (value: string) => void
}) {
  return (
    <Select value={value ?? ''} onValueChange={onChange}>
      <SelectTrigger className="w-[160px]">
        <SelectValue placeholder={`${label} axis`} />
      </SelectTrigger>
      <SelectContent>
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
