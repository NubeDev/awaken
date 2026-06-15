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
import { listCharts, type SavedChart } from '../../api/charts'
import {
  createBoard,
  deleteBoard,
  listBoards,
  updateBoard,
  type BoardPanel,
  type SavedBoard,
} from '../../api/boards'
import { AdminLayout } from '../../components/admin/AdminLayout'
import { Button } from '../../components/ui/button'
import { Input } from '../../components/ui/input'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '../../components/ui/select'
import { DashboardGrid } from '../../components/dashboards/DashboardGrid'
import { EmptyView } from '../../components/ui/StateView'

const route = getRouteApi('/t/$tenant/admin/dashboards')

export function AdminDashboards() {
  const { tenant } = route.useParams()
  const api = useApi(tenant)
  const qc = useQueryClient()

  const [selectedId, setSelectedId] = useState<string | null>(null)
  const [newName, setNewName] = useState('')
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

  function removePanel(chartId: string) {
    persist(panels.filter((p) => p.chart_id !== chartId))
  }

  const availableCharts = (charts.data ?? []).filter((c) => !panels.some((p) => p.chart_id === c.id))

  return (
    <AdminLayout active="dashboards">
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
              <Select value="" onValueChange={addPanel}>
                <SelectTrigger className="ml-auto w-[200px]">
                  <span className="flex items-center gap-1.5">
                    <Plus size={14} /> <SelectValue placeholder="Add chart" />
                  </span>
                </SelectTrigger>
                <SelectContent>
                  {availableCharts.length === 0 ? (
                    <SelectItem value="__none__" disabled>
                      No more charts
                    </SelectItem>
                  ) : (
                    availableCharts.map((c) => (
                      <SelectItem key={c.id} value={c.id}>
                        {c.name}
                      </SelectItem>
                    ))
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
          />
        )}
      </div>
    </AdminLayout>
  )
}
