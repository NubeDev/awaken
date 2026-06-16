// Admin · Dashboards — two real routes over the boards/charts record surface (§2,
// LAMINAR-BORROW.md):
//
//   • /admin/dashboards            → the directory: a table of every board (name,
//     panel count, last updated) with create / open / delete.
//   • /admin/dashboards/$boardId   → the builder: a draggable palette rail + the
//     responsive grid + a popover time-range picker. Pin charts by dragging a
//     palette tile onto the grid (or clicking it); drag/resize to arrange; the
//     layout persists debounced (one audited gate write per settle).
//
// The open board lives in the URL, not React state, so a board deep-links,
// survives refresh, and works with browser back/forward. Boards and charts are
// records (kind:"board"/"chart"), so every edit is audited/undoable.

import { getRouteApi, useNavigate } from '@tanstack/react-router'
import { useEffect, useMemo, useRef, useState } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { ArrowLeft, LayoutDashboard, Plus, SlidersHorizontal, Trash2 } from 'lucide-react'
import { formatDistanceToNow } from 'date-fns'
import { useApi } from '../../../api/ConnectionContext'
import { createChart, listCharts, type SavedChart } from '../../../api/charts'
import {
  createBoard,
  deleteBoard,
  listBoards,
  updateBoard,
  type BoardPanel,
  type BoardVariable,
  type SavedBoard,
} from '../../../api/boards'
import { usePageHeader } from '../../../components/shell/page-header'
import { Button } from '../../../components/ui/button'
import { Input } from '../../../components/ui/input'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '../../../components/ui/table'
import { DashboardGrid } from '../../../components/dashboards/DashboardGrid'
import { VariableBar } from '../../../components/dashboards/VariableBar'
import { VariableEditorDialog } from '../../../components/dashboards/VariableEditorDialog'
import { useBoardVariables } from '../../../components/dashboards/useBoardVariables'
import { varSearchKey, type Selection } from '../../../components/dashboards/board-variables'
import { getNavNode } from '../../../api/nav'
import { BoardPalette } from '../../../components/dashboards/BoardPalette'
import { TimeRangePicker } from '../../../components/dashboards/TimeRangePicker'
import { BoardRefresh } from '../../../components/dashboards/BoardRefresh'
import { DEFAULT_REFRESH, type RefreshInterval } from '../../../components/dashboards/board-refresh'
import { BLANK_CHART, CHART_PRESETS } from '../../../components/dashboards/chart-presets'
import {
  decodeDrag,
  PALETTE_DND_TYPE,
  type PaletteDrag,
} from '../../../components/dashboards/board-palette'
import {
  DEFAULT_RANGE,
  boardTimeScope,
  type BoardTimeRange as Range,
} from '../../../components/dashboards/board-params'
import { EmptyView, ErrorView, LoadingView } from '../../../components/ui/StateView'

const listRoute = getRouteApi('/t/$tenant/admin/dashboards')
const builderRoute = getRouteApi('/t/$tenant/admin/dashboards/$boardId')

// ── List view ─────────────────────────────────────────────────────────────────
// The dashboards directory: a table of every board. Opening one navigates to its
// builder route; creating one drops straight into it.
export function AdminDashboards() {
  const { tenant } = listRoute.useParams()
  const api = useApi(tenant)
  const qc = useQueryClient()
  const navigate = useNavigate()

  const [newName, setNewName] = useState('')
  const boards = useQuery({
    queryKey: ['boards', tenant],
    queryFn: () => listBoards(api),
  })

  function open(boardId: string) {
    navigate({
      to: '/t/$tenant/admin/dashboards/$boardId',
      params: { tenant, boardId },
    })
  }

  const create = useMutation({
    mutationFn: (name: string) => createBoard(api, name),
    onSuccess: (b) => {
      setNewName('')
      void qc.invalidateQueries({ queryKey: ['boards', tenant] })
      open(b.id) // open the new board straight into the builder
    },
  })

  const remove = useMutation({
    mutationFn: (id: string) => deleteBoard(api, id),
    onSuccess: () => void qc.invalidateQueries({ queryKey: ['boards', tenant] }),
  })

  usePageHeader({ crumbs: ['Admin', 'Dashboards'] })

  const list = boards.data ?? []
  return (
    <div className="px-6 py-6">
      <div className="mx-auto max-w-[1100px]">
        <div className="mb-6 flex items-center gap-3">
          <div className="grid size-11 place-items-center rounded-xl border border-border bg-card">
            <LayoutDashboard size={20} className="text-muted-foreground" />
          </div>
          <div>
            <h1 className="text-[22px] font-semibold tracking-tight">Dashboards</h1>
            <div className="text-[13px] text-muted-foreground">
              A board of saved charts, arranged on a draggable grid.
            </div>
          </div>
        </div>

        {/* Create row. */}
        <div className="mb-4 flex items-center gap-2">
          <Input
            value={newName}
            onChange={(e) => setNewName(e.target.value)}
            placeholder="New dashboard name"
            className="w-[260px]"
            onKeyDown={(e) => {
              if (e.key === 'Enter' && newName.trim()) create.mutate(newName.trim())
            }}
          />
          <Button
            onClick={() => newName.trim() && create.mutate(newName.trim())}
            disabled={create.isPending || !newName.trim()}
            className="gap-1.5"
          >
            <Plus size={15} /> Create dashboard
          </Button>
        </div>

        {boards.isPending ? (
          <LoadingView label="Loading dashboards…" />
        ) : boards.error ? (
          <ErrorView error={boards.error} />
        ) : list.length === 0 ? (
          <EmptyView
            title="No dashboards yet"
            hint="Create one above, then drag charts onto it from the palette."
          />
        ) : (
          <div className="overflow-hidden rounded-xl border border-border">
            <Table>
              <TableHeader>
                <TableRow className="hover:bg-transparent">
                  <TableHead>Name</TableHead>
                  <TableHead className="w-[120px]">Panels</TableHead>
                  <TableHead className="w-[180px]">Updated</TableHead>
                  <TableHead className="w-[140px] text-right">Actions</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {list.map((b) => (
                  <TableRow key={b.id} className="cursor-pointer" onClick={() => open(b.id)}>
                    <TableCell className="font-medium">{b.name}</TableCell>
                    <TableCell className="text-muted-foreground">{b.panels.length}</TableCell>
                    <TableCell className="text-muted-foreground">
                      {b.updated ? `${formatDistanceToNow(new Date(b.updated))} ago` : '—'}
                    </TableCell>
                    <TableCell className="text-right">
                      <div className="flex justify-end gap-1" onClick={(e) => e.stopPropagation()}>
                        <Button variant="outline" size="sm" onClick={() => open(b.id)}>
                          Open
                        </Button>
                        <Button
                          variant="ghost"
                          size="icon"
                          className="size-8 text-muted-foreground hover:text-destructive"
                          title="Delete dashboard"
                          onClick={() => remove.mutate(b.id)}
                          disabled={remove.isPending}
                        >
                          <Trash2 size={15} />
                        </Button>
                      </div>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </div>
        )}
      </div>
    </div>
  )
}

// ── Builder view ────────────────────────────────────────────────────────────────
// One board, identified by the `$boardId` route param. A board that doesn't exist
// (stale link, deleted) shows a not-found state with a way back to the directory.
export function AdminDashboardBuilder() {
  const { tenant, boardId } = builderRoute.useParams()
  const search = builderRoute.useSearch()
  const api = useApi(tenant)
  const qc = useQueryClient()
  const navigate = useNavigate()

  // Board-wide time range → a structured, UTC time scope sent to every panel (§5).
  const [range, setRange] = useState<Range>(DEFAULT_RANGE)
  const time = useMemo(() => boardTimeScope(range), [range])
  const [refresh, setRefresh] = useState<RefreshInterval>(DEFAULT_REFRESH)
  const [refreshing, setRefreshing] = useState(false)
  // Drop highlight for the empty-board placeholder (the grid manages its own).
  const [emptyDropActive, setEmptyDropActive] = useState(false)
  // A just-created blank chart whose editor the grid should auto-open (Grafana's
  // "add a generic chart and configure it in place").
  const [autoEditChartId, setAutoEditChartId] = useState<string | null>(null)
  // Local working copy of the board's panels — the grid edits this live; a
  // debounced effect flushes it to the gate so drags don't thrash the backend.
  const [panels, setPanels] = useState<BoardPanel[]>([])

  const boards = useQuery({
    queryKey: ['boards', tenant],
    queryFn: () => listBoards(api),
  })
  const charts = useQuery({
    queryKey: ['charts', tenant],
    queryFn: () => listCharts(api),
  })

  const board = useMemo<SavedBoard | undefined>(
    () => boards.data?.find((b) => b.id === boardId),
    [boards.data, boardId],
  )

  const chartMap = useMemo(() => {
    const m = new Map<string, SavedChart>()
    for (const c of charts.data ?? []) m.set(c.id, c)
    return m
  }, [charts.data])

  // Page context: a `?nav=<id>` deep-link binds the board to a navigation node's
  // context (the fleet story — one board, many mounts). Load the node and read its
  // `context.values` as the variable bindings (PAGE-CONTEXT-AND-NAV §1).
  const navNodeId = search.nav
  const navNode = useQuery({
    queryKey: ['nav-node', tenant, navNodeId],
    queryFn: () => getNavNode(api, navNodeId!),
    enabled: Boolean(navNodeId),
  })
  const navValues = useMemo<Record<string, Selection>>(
    () => (navNode.data?.context?.values ?? {}) as Record<string, Selection>,
    [navNode.data],
  )

  // Resolve the board's variables to live selections + the wire array. Bindings
  // come from the URL (`?var-*`) over the nav context over the board default.
  const vars = useBoardVariables({
    api,
    tenant,
    variables: board?.variables ?? [],
    time,
    navValues,
    search,
  })

  // Write a variable selection into the URL so it is shareable and survives a
  // refresh; an empty/default selection clears its key to keep the URL tidy.
  function setVar(name: string, selection: Selection) {
    navigate({
      to: '/t/$tenant/admin/dashboards/$boardId',
      params: { tenant, boardId },
      search: (prev) => {
        const next = { ...prev }
        const empty = selection === '' || (Array.isArray(selection) && selection.length === 0)
        // The URL carries strings; a scalar/array round-trips as its string form
        // (the backend lowers string values as quoted literals regardless).
        if (empty) delete next[varSearchKey(name)]
        else
          next[varSearchKey(name)] = Array.isArray(selection)
            ? selection.map(String)
            : String(selection)
        return next
      },
      replace: true,
    })
  }

  // Seed the working panels when the board loads/changes.
  useEffect(() => {
    if (board) setPanels(board.panels)
  }, [board?.id]) // eslint-disable-line react-hooks/exhaustive-deps

  function back() {
    navigate({ to: '/t/$tenant/admin/dashboards', params: { tenant } })
  }

  const save = useMutation({
    // Carry the board's variables through a layout write — they live in the same
    // record content, so omitting them would drop them on the next drag/resize.
    mutationFn: (next: BoardPanel[]) =>
      updateBoard(api, board!.id, {
        name: board!.name,
        panels: next,
        variables: board!.variables,
      }),
    onSuccess: () => void qc.invalidateQueries({ queryKey: ['boards', tenant] }),
  })

  // Save the board's variables (the editor's working list) alongside the current
  // panels. Invalidating the boards re-renders the bar; the board-batch re-runs so
  // panels pick up a changed/added variable.
  const [variablesOpen, setVariablesOpen] = useState(false)
  const saveVariables = useMutation({
    mutationFn: (next: BoardVariable[]) =>
      updateBoard(api, board!.id, { name: board!.name, panels, variables: next }),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['boards', tenant] })
      void qc.invalidateQueries({ queryKey: ['board-batch'] })
      setVariablesOpen(false)
    },
  })

  // Debounce layout writes: update local panels now, flush 600ms after the last
  // change — one gate write per settle.
  const flush = useRef<ReturnType<typeof setTimeout> | null>(null)
  function persist(next: BoardPanel[]) {
    setPanels(next)
    if (flush.current) clearTimeout(flush.current)
    flush.current = setTimeout(() => {
      if (board) save.mutate(next)
    }, 600)
  }

  const remove = useMutation({
    mutationFn: (id: string) => deleteBoard(api, id),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['boards', tenant] })
      back()
    },
  })

  function addPanel(chartId: string) {
    if (panels.some((p) => p.chart_id === chartId)) return
    const i = panels.length
    persist([
      ...panels,
      {
        chart_id: chartId,
        x: (i % 2) * 6,
        y: Math.floor(i / 2) * 4,
        w: 6,
        h: 4,
      },
    ])
  }

  // Materialise a preset into a kind:"chart" record, then place it on the board.
  const addPreset = useMutation({
    mutationFn: (presetName: string) => {
      const preset = CHART_PRESETS.find((p) => p.name === presetName)
      if (!preset) throw new Error(`unknown preset: ${presetName}`)
      return createChart(api, {
        name: preset.name,
        sql: preset.sql,
        config: preset.config,
      })
    },
    onSuccess: (c) => {
      void qc.invalidateQueries({ queryKey: ['charts', tenant] })
      addPanel(c.id)
    },
  })

  // Add a generic, blank chart directly on the board: create the record, place it,
  // and flag it so the grid pops the in-place editor open immediately. No trip
  // through the Query console — author the SQL/type/columns right on the board.
  const addBlank = useMutation({
    mutationFn: () => createChart(api, BLANK_CHART),
    onSuccess: (c) => {
      void qc.invalidateQueries({ queryKey: ['charts', tenant] })
      addPanel(c.id)
      setAutoEditChartId(c.id)
    },
  })

  // The single add path for both the palette drag/drop and click-to-add.
  function onAddDrag(drag: PaletteDrag) {
    if (drag.source === 'preset') addPreset.mutate(drag.preset)
    else addPanel(drag.chartId)
  }

  function removePanel(chartId: string) {
    persist(panels.filter((p) => p.chart_id !== chartId))
  }

  const availableCharts = (charts.data ?? []).filter(
    (c) => !panels.some((p) => p.chart_id === c.id),
  )

  usePageHeader({ crumbs: ['Admin', 'Dashboards', board?.name ?? '…'] })

  // A board still loading vs. one that genuinely doesn't exist (stale/deleted link).
  if (boards.isPending) return <LoadingView label="Loading dashboard…" />
  if (boards.error) return <ErrorView error={boards.error} />
  if (!board) {
    return (
      <div className="px-6 py-6">
        <div className="mx-auto max-w-[1100px] space-y-4">
          <Button variant="ghost" size="sm" onClick={back} className="gap-1.5">
            <ArrowLeft size={15} /> All dashboards
          </Button>
          <EmptyView
            title="Dashboard not found"
            hint="It may have been deleted. Pick one from the list."
          />
        </div>
      </div>
    )
  }

  return (
    <div className="flex h-full flex-col px-6 py-6">
      <div className="mb-4 flex flex-wrap items-center gap-3">
        <Button variant="ghost" size="sm" onClick={back} className="gap-1.5">
          <ArrowLeft size={15} /> All dashboards
        </Button>
        <div className="flex items-center gap-2">
          <div className="grid size-8 place-items-center rounded-lg border border-border bg-card">
            <LayoutDashboard size={15} className="text-muted-foreground" />
          </div>
          <h1 className="text-[17px] font-semibold tracking-tight">{board.name}</h1>
        </div>

        <div className="ml-auto flex flex-wrap items-center gap-2">
          <Button
            size="sm"
            onClick={() => addBlank.mutate()}
            disabled={addBlank.isPending}
            className="gap-1.5"
          >
            <Plus size={15} /> Add chart
          </Button>
          <Button
            variant="outline"
            size="sm"
            onClick={() => setVariablesOpen(true)}
            className="gap-1.5"
          >
            <SlidersHorizontal size={15} /> Variables
            {board.variables.length > 0 && (
              <span className="rounded bg-muted px-1 text-[11px] text-muted-foreground">
                {board.variables.length}
              </span>
            )}
          </Button>
          <TimeRangePicker value={range} onChange={setRange} />
          <BoardRefresh value={refresh} onChange={setRefresh} refreshing={refreshing} />
          <Button
            variant="ghost"
            size="sm"
            onClick={() => remove.mutate(board.id)}
            disabled={remove.isPending}
            className="gap-1.5 text-muted-foreground"
          >
            <Trash2 size={15} /> Delete
          </Button>
        </div>
      </div>

      <div className="grid min-h-0 flex-1 gap-4 lg:grid-cols-[240px_1fr]">
        <BoardPalette charts={availableCharts} onAdd={onAddDrag} />

        <div className="min-h-0 overflow-y-auto pe-1">
          {vars.visible.length > 0 && (
            <VariableBar
              variables={vars.visible}
              options={vars.options}
              selections={vars.selections}
              onChange={setVar}
            />
          )}
          {panels.length === 0 ? (
            // An empty board still needs to be a drop target — otherwise the
            // palette's "drag a tile onto the board" affordance has nothing to
            // catch it (the grid only mounts once there's a panel).
            <div
              onDragOver={(e) => {
                e.preventDefault()
                e.dataTransfer.dropEffect = 'copy'
                if (!emptyDropActive) setEmptyDropActive(true)
              }}
              onDragLeave={(e) => {
                if (e.currentTarget === e.target) setEmptyDropActive(false)
              }}
              onDrop={(e) => {
                e.preventDefault()
                setEmptyDropActive(false)
                const drag = decodeDrag(e.dataTransfer.getData(PALETTE_DND_TYPE))
                if (drag) onAddDrag(drag)
              }}
              className={
                'grid h-full min-h-[260px] place-items-center rounded-xl border-2 border-dashed transition-colors ' +
                (emptyDropActive ? 'border-primary/50 bg-primary/[0.06]' : 'border-border')
              }
            >
              <div className="max-w-xs text-center">
                <div className="text-[14px] font-semibold">Empty board</div>
                <div className="mt-1 text-[12.5px] text-muted-foreground">
                  Drag a tile from the palette onto the board, click it to add, or add a generic
                  chart and configure it here.
                </div>
                <Button
                  size="sm"
                  onClick={() => addBlank.mutate()}
                  disabled={addBlank.isPending}
                  className="mt-3 gap-1.5"
                >
                  <Plus size={15} /> Add chart
                </Button>
              </div>
            </div>
          ) : (
            <DashboardGrid
              tenant={tenant}
              panels={panels}
              charts={chartMap}
              onLayoutChange={persist}
              onRemovePanel={removePanel}
              onAddDrag={onAddDrag}
              autoEditChartId={autoEditChartId}
              onAutoEditConsumed={() => setAutoEditChartId(null)}
              time={time}
              variables={vars.queryVariables}
              varRevision={vars.revision}
              refresh={refresh}
              onFetchingChange={setRefreshing}
            />
          )}
        </div>
      </div>

      <VariableEditorDialog
        open={variablesOpen}
        onOpenChange={setVariablesOpen}
        boardName={board.name}
        variables={board.variables}
        onSave={(next) => saveVariables.mutate(next)}
        saving={saveVariables.isPending}
      />
    </div>
  )
}
