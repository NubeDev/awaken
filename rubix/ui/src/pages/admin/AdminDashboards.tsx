// Admin · Dashboards — pinned boards of saved charts on a draggable grid (§2,
// LAMINAR-BORROW.md). Boards and charts are records (kind:"board"/"chart"), so
// they ride the gate; layout changes persist debounced (one audited write per
// settle). Charts are authored in the Query console ("Save as chart"); here they
// are placed, arranged, and resized.

import { getRouteApi } from '@tanstack/react-router'
import { useEffect, useMemo, useRef, useState } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { LayoutDashboard, Plus, Trash2 } from 'lucide-react'
import { useApi } from '../../api/ConnectionContext'
import { createChart, listCharts, type SavedChart } from '../../api/charts'
import {
  createBoard,
  deleteBoard,
  listBoards,
  updateBoard,
  type BoardPanel,
  type SavedBoard,
} from '../../api/boards'
import { usePageHeader } from '../../components/shell/page-header'
import { Button } from '../../components/ui/button'
import { Input } from '../../components/ui/input'
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '../../components/ui/select'
import { DashboardGrid } from '../../components/dashboards/DashboardGrid'
import { BoardTimeRange } from '../../components/dashboards/BoardTimeRange'
import { BoardRefresh } from '../../components/dashboards/BoardRefresh'
import { DEFAULT_REFRESH, type RefreshInterval } from '../../components/dashboards/board-refresh'
import { CHART_PRESETS, type ChartPreset, type PresetGroup } from '../../components/dashboards/chart-presets'
import { DEFAULT_RANGE, boardTimeScope, type BoardTimeRange as Range } from '../../components/dashboards/board-params'
import { EmptyView } from '../../components/ui/StateView'

const PRESET_GROUPS: PresetGroup[] = ['Records', 'Audit', 'Traces']

const route = getRouteApi('/t/$tenant/admin/dashboards')

export function AdminDashboards() {
  const { tenant } = route.useParams()
  const api = useApi(tenant)
  const qc = useQueryClient()

  const [selectedId, setSelectedId] = useState<string | null>(null)
  const [newName, setNewName] = useState('')
  // Board-wide time range — sent to every panel as a structured, UTC time scope
  // so one control re-scopes the whole board (§5). The backend injects the
  // window/bucket by expanding each chart's time macros; the client no longer
  // formats a locale datetime into SQL (the old timezone bug). Memoised so panels
  // only refetch when the window actually changes.
  const [range, setRange] = useState<Range>(DEFAULT_RANGE)
  const time = useMemo(() => boardTimeScope(range), [range])
  // Board auto-refresh interval (§6); drives refetchInterval on the batch query.
  const [refresh, setRefresh] = useState<RefreshInterval>(DEFAULT_REFRESH)
  // Whether a batch refetch is in flight — spins the refresh icon.
  const [refreshing, setRefreshing] = useState(false)
  // Local working copy of the open board's panels — the grid edits this live; a
  // debounced effect flushes it to the gate so drags don't thrash the backend.
  const [panels, setPanels] = useState<BoardPanel[]>([])

  const boards = useQuery({ queryKey: ['boards', tenant], queryFn: () => listBoards(api) })
  const charts = useQuery({ queryKey: ['charts', tenant], queryFn: () => listCharts(api) })

  const selected = useMemo<SavedBoard | undefined>(
    () => boards.data?.find((b) => b.id === selectedId),
    [boards.data, selectedId],
  )

  const chartMap = useMemo(() => {
    const m = new Map<string, SavedChart>()
    for (const c of charts.data ?? []) m.set(c.id, c)
    return m
  }, [charts.data])

  // Seed the working panels when the selected board loads/changes.
  useEffect(() => {
    if (selected) setPanels(selected.panels)
  }, [selected?.id]) // eslint-disable-line react-hooks/exhaustive-deps

  // Default the selection to the first board once they load.
  useEffect(() => {
    if (selectedId === null && boards.data && boards.data.length > 0) {
      setSelectedId(boards.data[0].id)
    }
  }, [boards.data, selectedId])

  const save = useMutation({
    mutationFn: (next: BoardPanel[]) =>
      updateBoard(api, selected!.id, { name: selected!.name, panels: next }),
    onSuccess: () => void qc.invalidateQueries({ queryKey: ['boards', tenant] }),
  })

  // Debounce layout writes: update local panels now, flush 600ms after the last
  // change — one gate write per settle (PRODUCT-UI's debounced PATCH).
  const flush = useRef<ReturnType<typeof setTimeout> | null>(null)
  function persist(next: BoardPanel[]) {
    setPanels(next)
    if (flush.current) clearTimeout(flush.current)
    flush.current = setTimeout(() => {
      if (selected) save.mutate(next)
    }, 600)
  }

  const create = useMutation({
    mutationFn: (name: string) => createBoard(api, name),
    onSuccess: (b) => {
      setNewName('')
      setSelectedId(b.id)
      void qc.invalidateQueries({ queryKey: ['boards', tenant] })
    },
  })

  const remove = useMutation({
    mutationFn: (id: string) => deleteBoard(api, id),
    onSuccess: () => {
      setSelectedId(null)
      void qc.invalidateQueries({ queryKey: ['boards', tenant] })
    },
  })

  function addPanel(chartId: string) {
    if (panels.some((p) => p.chart_id === chartId)) return
    // Place two-per-row, stacking downward.
    const i = panels.length
    persist([...panels, { chart_id: chartId, x: (i % 2) * 6, y: Math.floor(i / 2) * 4, w: 6, h: 4 }])
  }

  // One-click preset: materialise the preset as a kind:"chart" record, then place
  // it on the board. Time-series presets use the `$__timeBucket`/`$__timeFilter`
  // macros, so the backend scopes them to the board range the moment they land.
  const addPreset = useMutation({
    mutationFn: (preset: ChartPreset) =>
      createChart(api, { name: preset.name, sql: preset.sql, config: preset.config }),
    onSuccess: (c) => {
      void qc.invalidateQueries({ queryKey: ['charts', tenant] })
      addPanel(c.id)
    },
  })

  function onPick(value: string) {
    const preset = CHART_PRESETS.find((p) => p.name === value)
    if (preset) addPreset.mutate(preset)
    else addPanel(value)
  }

  function removePanel(chartId: string) {
    persist(panels.filter((p) => p.chart_id !== chartId))
  }

  const availableCharts = (charts.data ?? []).filter((c) => !panels.some((p) => p.chart_id === c.id))

  usePageHeader({ crumbs: ['Admin', 'Dashboards'] })

  return (
    <div className="px-6 py-6">
      <div className="mx-auto max-w-[1280px]">
        <div className="mb-5 flex items-center gap-3">
          <div className="grid size-11 place-items-center rounded-xl border border-border bg-card">
            <LayoutDashboard size={20} className="text-muted-foreground" />
          </div>
          <div>
            <h1 className="text-[22px] font-semibold tracking-tight">Dashboards</h1>
            <div className="text-[13px] text-muted-foreground">
              Pin saved charts to a board, drag and resize to arrange.
            </div>
          </div>
        </div>

        {/* Board bar: select, create, delete. */}
        <div className="mb-4 flex flex-wrap items-center gap-2">
          <Select value={selectedId ?? ''} onValueChange={setSelectedId}>
            <SelectTrigger className="w-[220px]">
              <SelectValue placeholder="Select a board" />
            </SelectTrigger>
            <SelectContent>
              {(boards.data ?? []).map((b) => (
                <SelectItem key={b.id} value={b.id}>
                  {b.name}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <Input
            value={newName}
            onChange={(e) => setNewName(e.target.value)}
            placeholder="New board name"
            className="w-[200px]"
            onKeyDown={(e) => {
              if (e.key === 'Enter' && newName.trim()) create.mutate(newName.trim())
            }}
          />
          <Button
            variant="outline"
            onClick={() => newName.trim() && create.mutate(newName.trim())}
            disabled={create.isPending}
            className="gap-1.5"
          >
            <Plus size={15} /> Create
          </Button>

          {selected && (
            <>
              <Select value="" onValueChange={onPick}>
                <SelectTrigger className="ml-auto w-[200px]">
                  <span className="flex items-center gap-1.5">
                    <Plus size={14} /> <SelectValue placeholder="Add chart" />
                  </span>
                </SelectTrigger>
                <SelectContent>
                  {/* Presets — one-click charts, grouped by surface. */}
                  {PRESET_GROUPS.map((group) => (
                    <SelectGroup key={group}>
                      <div className="px-2 py-1 text-[11px] font-medium uppercase tracking-wide text-muted-foreground">
                        {group}
                      </div>
                      {CHART_PRESETS.filter((p) => p.group === group).map((p) => (
                        <SelectItem key={p.name} value={p.name}>
                          {p.name}
                        </SelectItem>
                      ))}
                    </SelectGroup>
                  ))}
                  {/* Saved charts authored in the Query console. */}
                  {availableCharts.length > 0 && (
                    <SelectGroup>
                      <div className="px-2 py-1 text-[11px] font-medium uppercase tracking-wide text-muted-foreground">
                        Saved charts
                      </div>
                      {availableCharts.map((c) => (
                        <SelectItem key={c.id} value={c.id}>
                          {c.name}
                        </SelectItem>
                      ))}
                    </SelectGroup>
                  )}
                </SelectContent>
              </Select>
              <Button
                variant="ghost"
                onClick={() => remove.mutate(selected.id)}
                disabled={remove.isPending}
                className="gap-1.5 text-muted-foreground"
              >
                <Trash2 size={15} /> Delete board
              </Button>
            </>
          )}
        </div>

        {/* Board time range + auto-refresh — one control re-scopes every
            parameterised panel; the other drives visibility-aware polling (§6). */}
        {selected && panels.length > 0 && (
          <div className="mb-4 flex flex-wrap items-center gap-x-4 gap-y-2">
            <BoardTimeRange value={range} onChange={setRange} />
            <BoardRefresh value={refresh} onChange={setRefresh} refreshing={refreshing} />
          </div>
        )}

        {!selected ? (
          <EmptyView
            title="No board selected"
            hint={
              (boards.data?.length ?? 0) === 0
                ? 'Create a board, then add charts saved from the Query console.'
                : 'Pick a board above.'
            }
          />
        ) : panels.length === 0 ? (
          <EmptyView
            title="Empty board"
            hint={
              (charts.data?.length ?? 0) === 0
                ? 'Save a chart in the Query console first ("Save as chart"), then add it here.'
                : 'Use "Add chart" to place a saved chart.'
            }
          />
        ) : (
          <DashboardGrid
            tenant={tenant}
            panels={panels}
            charts={chartMap}
            onLayoutChange={persist}
            onRemovePanel={removePanel}
            time={time}
            refresh={refresh}
            onFetchingChange={setRefreshing}
          />
        )}
      </div>
    </div>
  )
}
