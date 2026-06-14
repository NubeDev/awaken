import { useCallback, useMemo, useRef, useState } from 'react'
import {
  addEdge,
  Background,
  BackgroundVariant,
  Controls,
  ReactFlow,
  ReactFlowProvider,
  useEdgesState,
  useNodesState,
  useReactFlow,
  type Connection,
  type Edge,
  type Node,
} from '@xyflow/react'
import '@xyflow/react/dist/style.css'
import { toast } from 'sonner'
import {
  useBoardComponents,
  useBoardOutputsStream,
  useBoards,
  useRunInlineBoard,
  useSaveBoard,
} from '@/api/hooks'
import { useScope } from '@/context/scope-provider'
import type {
  BoardGraph,
  BoardView,
  ComponentView,
  RunBoardResponse,
} from '@/api/types'
import { Main } from '@/components/layout/main'
import { PageHeader } from '@/components/layout/page-header'
import { Card } from '@/components/ui/card'
import { Plus } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { BoardPicker } from './components/board-picker'
import { BoardStatusBar } from './components/board-status-bar'
import { FlowNode, type FlowNodeData } from './components/flow-node'
import { NewBoardDialog } from './components/new-board-dialog'
import { NodeInspector } from './components/node-inspector'
import { DRAG_TYPE, NodePalette } from './components/node-palette'
import { RunOutput } from './components/run-output'
import {
  edgeId,
  graphFromFlow,
  indexComponents,
  inportType,
  mapBoard,
  outportType,
  portAccepts,
  type ComponentIndex,
} from './data/map-board'
import { newNode } from './data/new-node'

export function Flows() {
  return (
    <ReactFlowProvider>
      <FlowsInner />
    </ReactFlowProvider>
  )
}

function FlowsInner() {
  // Flows live under a site route; they belong to this org+site (org-level flows
  // also surface, since the list returns the org's flows at every scope).
  const { org, site } = useScope()
  const boardsQuery = useBoards(org, site?.id)
  const componentsQuery = useBoardComponents()

  const boards = useMemo(() => boardsQuery.data ?? [], [boardsQuery.data])
  const componentIndex = useMemo<ComponentIndex>(
    () => indexComponents(componentsQuery.data ?? []),
    [componentsQuery.data]
  )

  // No explicit selection yet means "the first board".
  const [pickedSlug, setPickedSlug] = useState<string | undefined>()
  const [newOpen, setNewOpen] = useState(false)
  const selectedSlug = pickedSlug ?? boards[0]?.slug
  const board = useMemo(
    () => boards.find((b) => b.slug === selectedSlug),
    [boards, selectedSlug]
  )

  const ready = Boolean(board && componentsQuery.data)
  const newFlowDialog = (
    <NewBoardDialog
      open={newOpen}
      onOpenChange={setNewOpen}
      existingSlugs={boards.map((b) => b.slug)}
      onCreated={(created) => setPickedSlug(created.slug)}
    />
  )

  return (
    <>
      <PageHeader title='Flow Boards' sub='reflow control & analytics graphs' />
      <Main fluid fixed className='flex min-h-0'>
        {ready && board ? (
          // Key on slug so swapping boards remounts the editor with fresh
          // canvas state derived from the new board's stored graph.
          <BoardEditor
            key={board.slug}
            board={board}
            componentIndex={componentIndex}
            components={componentsQuery.data ?? []}
            boards={boards}
            selectedSlug={selectedSlug}
            onSelectBoard={setPickedSlug}
            onNewFlow={() => setNewOpen(true)}
            newFlowDialog={newFlowDialog}
          />
        ) : (
          <div className='grid min-h-0 w-full flex-1 gap-3 lg:grid-cols-[200px_1fr_230px]'>
            <Card className='scroll overflow-y-auto p-2.5'>
              <NodePalette components={componentsQuery.data ?? []} />
            </Card>
            <div className='flex min-h-0 flex-col'>
              <BoardBar
                boards={boards}
                selectedSlug={selectedSlug}
                onSelect={setPickedSlug}
                onNewFlow={() => setNewOpen(true)}
              />
              <div className='border-border mt-2.5 min-h-0 flex-1 overflow-hidden rounded-lg border'>
                <FlowsPlaceholder
                  loading={boardsQuery.isLoading || componentsQuery.isLoading}
                  error={boardsQuery.error ?? componentsQuery.error}
                  empty={boards.length === 0}
                />
              </div>
            </div>
            <Card className='scroll overflow-y-auto p-3'>
              <NodeInspector node={undefined} onConfigChange={() => {}} onDelete={() => {}} />
            </Card>
            {newFlowDialog}
          </div>
        )}
      </Main>
    </>
  )
}

type BoardEditorProps = {
  board: BoardView
  componentIndex: ComponentIndex
  components: ComponentView[]
  boards: BoardView[]
  selectedSlug: string | undefined
  onSelectBoard: (slug: string) => void
  onNewFlow: () => void
  /** Rendered once at the editor root so the create dialog has a mount point. */
  newFlowDialog: React.ReactNode
}

/** The board switcher row: picker plus a "New flow" action, in both states. */
function BoardBar({
  boards,
  selectedSlug,
  onSelect,
  onNewFlow,
}: {
  boards: BoardView[]
  selectedSlug: string | undefined
  onSelect: (slug: string) => void
  onNewFlow: () => void
}) {
  return (
    <div className='flex items-center gap-2'>
      <BoardPicker boards={boards} selectedSlug={selectedSlug} onSelect={onSelect} />
      <Button variant='outline' size='sm' onClick={onNewFlow}>
        <Plus className='size-3.5' /> New flow
      </Button>
    </div>
  )
}

/**
 * The authorable editor for one board: the full three-column surface (palette,
 * canvas, inspector). Nodes/edges are controlled so the operator can drag,
 * connect, configure, and delete; Save persists the canvas as a new board
 * version. Mounted with a slug key so its state seeds once from the stored
 * graph.
 */
function BoardEditor({
  board,
  componentIndex,
  components,
  boards,
  selectedSlug,
  onSelectBoard,
  onNewFlow,
  newFlowDialog,
}: BoardEditorProps) {
  const nodeTypes = useMemo(() => ({ block: FlowNode }), [])
  const runBoard = useRunInlineBoard()
  const saveBoard = useSaveBoard()
  const { screenToFlowPosition } = useReactFlow()

  const seed = useMemo(
    () => mapBoard(board.graph, componentIndex),
    [board.graph, componentIndex]
  )
  const [nodes, setNodes, onNodesChange] = useNodesState(seed.nodes)
  const [edges, setEdges, onEdgesChange] = useEdgesState(seed.edges)
  const [selectedId, setSelectedId] = useState<string | undefined>()
  const [lastRun, setLastRun] = useState<RunBoardResponse | undefined>()
  const [dirty, setDirty] = useState(false)
  const markDirty = useCallback(() => setDirty(true), [])

  const wrapperRef = useRef<HTMLDivElement>(null)
  const selectedNode = nodes.find((n) => n.id === selectedId)?.data as
    | FlowNodeData
    | undefined

  // A board that the scheduler runs autonomously (enabled + non-manual) streams
  // its node values in real time (SSE), so the canvas shows live values without
  // a manual Test Run. A manual board has no background runs, so we don't stream
  // it.
  const isLive = board.enabled && board.trigger.kind !== 'manual'
  const outputsStream = useBoardOutputsStream(
    board.slug,
    { org: board.org, siteId: board.site_id ?? undefined },
    isLive
  )

  // Live values, keyed by node, from the stream. Suppressed while the user has
  // unsaved edits — then a manual Test Run owns the values (it writes
  // `lastValues` onto the nodes directly) and the background stream must not
  // fight it. Empty otherwise, so nodes show their own (Test Run) values or none.
  const liveByNode = useMemo(() => {
    const map = new Map<string, Record<string, unknown>>()
    if (!isLive || dirty || !outputsStream.data) return map
    for (const o of outputsStream.data) {
      const m = map.get(o.node) ?? {}
      m[o.port] = o.value
      map.set(o.node, m)
    }
    return map
  }, [isLive, dirty, outputsStream.data])

  // Per-node link quality from the stream, keyed like `liveByNode`, so the canvas
  // can colour a fault/null port differently from a good value.
  const liveQualityByNode = useMemo(() => {
    const map = new Map<string, Record<string, string>>()
    if (!isLive || dirty || !outputsStream.data) return map
    for (const o of outputsStream.data) {
      if (!o.quality) continue
      const m = map.get(o.node) ?? {}
      m[o.port] = o.quality
      map.set(o.node, m)
    }
    return map
  }, [isLive, dirty, outputsStream.data])

  // The nodes React Flow renders: the edited nodes overlaid with live values
  // (and their quality) when streaming. Derived (not stored) so streaming never
  // disturbs drag/edit state, and the overlay vanishes the moment the board goes
  // manual/disabled.
  const displayNodes = useMemo(() => {
    if (liveByNode.size === 0) return nodes
    return nodes.map((n) => ({
      ...n,
      data: {
        ...n.data,
        lastValues: liveByNode.get(n.id) ?? n.data.lastValues,
        lastQuality: liveQualityByNode.get(n.id) ?? n.data.lastQuality,
      },
    }))
  }, [nodes, liveByNode, liveQualityByNode])

  // The inspector's value panel reads from the live poll when live, else from
  // the last manual Test Run.
  const inspectorRun = useMemo(() => {
    if (liveByNode.size === 0) return lastRun
    return {
      outputs: (outputsStream.data ?? []).map((o) => ({
        node: o.node,
        port: o.port,
        value: o.value,
      })),
    }
  }, [liveByNode, outputsStream.data, lastRun])

  // Port-type compatibility, checked against the live schema. Used both to gate
  // the drag interaction (React Flow's `isValidConnection`) and to guard the
  // committed connection, so the editor never produces a wire the engine would
  // reject at load.
  const isValidConnection = useCallback(
    (c: Connection | Edge) => {
      const source = outportType(
        nodes.find((n) => n.id === c.source),
        c.sourceHandle
      )
      const target = inportType(
        nodes.find((n) => n.id === c.target),
        c.targetHandle
      )
      if (!source || !target) return false
      return portAccepts(source, target)
    },
    [nodes]
  )

  const onConnect = useCallback(
    (c: Connection) => {
      if (!isValidConnection(c)) {
        toast.error('Incompatible ports', {
          description: 'That output type cannot feed that input.',
        })
        return
      }
      setEdges((eds) =>
        addEdge(
          {
            ...c,
            id: edgeId({
              from_node: c.source ?? '',
              from_port: c.sourceHandle ?? '',
              to_node: c.target ?? '',
              to_port: c.targetHandle ?? '',
            }),
            animated: true,
          },
          eds
        )
      )
      markDirty()
    },
    [isValidConnection, setEdges, markDirty]
  )

  const onDrop = useCallback(
    (event: React.DragEvent) => {
      event.preventDefault()
      const component = event.dataTransfer.getData(DRAG_TYPE)
      const schema = componentIndex.get(component)
      if (!schema) return
      const position = screenToFlowPosition({ x: event.clientX, y: event.clientY })
      setNodes((nds) => {
        const node = newNode(schema, position, new Set(nds.map((n) => n.id)))
        setSelectedId(node.id)
        return [...nds, node]
      })
      markDirty()
    },
    [componentIndex, screenToFlowPosition, setNodes, markDirty]
  )

  const onDragOver = useCallback((event: React.DragEvent) => {
    event.preventDefault()
    event.dataTransfer.dropEffect = 'move'
  }, [])

  const onConfigChange = useCallback(
    (key: string, value: unknown) => {
      if (!selectedId) return
      setNodes((nds) =>
        nds.map((n) => {
          if (n.id !== selectedId) return n
          const config = { ...n.data.config }
          if (value === undefined) delete config[key]
          else config[key] = value
          return { ...n, data: { ...n.data, config } }
        })
      )
      markDirty()
    },
    [selectedId, setNodes, markDirty]
  )

  const onDeleteNode = useCallback(() => {
    if (!selectedId) return
    setNodes((nds) => nds.filter((n) => n.id !== selectedId))
    setEdges((eds) => eds.filter((e) => e.source !== selectedId && e.target !== selectedId))
    setSelectedId(undefined)
    markDirty()
  }, [selectedId, setNodes, setEdges, markDirty])

  // Test Run evaluates the live canvas (unsaved edits included), so what you
  // see is what runs — no "save first" surprise. Uses the inline run endpoint.
  const onTestRun = useCallback(() => {
    const graph = graphFromFlow(nodes as Node<FlowNodeData>[], edges)
    if (graph.nodes.length === 0) {
      toast('Nothing to run', { description: 'Add a node to the canvas first.' })
      return
    }
    runBoard.mutate(graph, {
      onSuccess: (res) => {
        setLastRun(res)
        // Fold the run's outputs onto each node so its ports show live values.
        // A node absent from this run gets its values cleared, so the canvas
        // never shows stale numbers from an earlier run.
        const byNode = new Map<string, Record<string, unknown>>()
        for (const o of res.outputs) {
          const m = byNode.get(o.node) ?? {}
          m[o.port] = o.value
          byNode.set(o.node, m)
        }
        setNodes((nds) =>
          nds.map((n) => ({ ...n, data: { ...n.data, lastValues: byNode.get(n.id) } }))
        )
        toast('Board run complete', {
          description: `${res.outputs.length} outport packet${res.outputs.length === 1 ? '' : 's'} produced`,
        })
      },
      onError: (err) => toast.error('Board run failed', { description: String(err) }),
    })
  }, [nodes, edges, runBoard, setNodes])

  const onSave = useCallback(() => {
    const graph: BoardGraph = graphFromFlow(nodes as Node<FlowNodeData>[], edges)
    saveBoard.mutate(
      {
        org: board.org,
        site_id: board.site_id ?? null,
        slug: board.slug,
        display_name: board.display_name,
        enabled: board.enabled,
        trigger: board.trigger,
        board: graph,
      },
      {
        onSuccess: (saved) => {
          setDirty(false)
          toast('Board saved', { description: `${saved.slug} · v${saved.version}` })
        },
        onError: (err) => toast.error('Save failed', { description: String(err) }),
      }
    )
  }, [board, nodes, edges, saveBoard])

  return (
    <div className='grid min-h-0 w-full flex-1 gap-3 lg:grid-cols-[200px_1fr_230px]'>
      <Card className='scroll overflow-y-auto p-2.5'>
        <NodePalette components={components} />
      </Card>

      <div className='flex min-h-0 flex-col'>
        <div className='pb-2.5'>
          <BoardBar
            boards={boards}
            selectedSlug={selectedSlug}
            onSelect={onSelectBoard}
            onNewFlow={onNewFlow}
          />
        </div>
        <BoardStatusBar
          board={board}
          slug={board.slug}
          name={board.display_name}
          version={board.version}
          enabled={board.enabled}
          intervalSeconds={
            board.trigger.kind === 'interval' ? board.trigger.seconds : undefined
          }
          nodeCount={nodes.length}
          edgeCount={edges.length}
          running={runBoard.isPending}
          onTestRun={onTestRun}
          dirty={dirty}
          saving={saveBoard.isPending}
          onSave={onSave}
          onDeleted={() => onSelectBoard('')}
        />
        <div
          ref={wrapperRef}
          className='border-border min-h-0 flex-1 overflow-hidden rounded-lg border'
          onDrop={onDrop}
          onDragOver={onDragOver}
        >
          <ReactFlow
            nodes={displayNodes}
            edges={edges}
            nodeTypes={nodeTypes}
            onNodesChange={(changes) => {
              onNodesChange(changes)
              if (changes.some((c) => c.type === 'position' || c.type === 'remove')) markDirty()
            }}
            onEdgesChange={(changes) => {
              onEdgesChange(changes)
              if (changes.some((c) => c.type === 'remove')) markDirty()
            }}
            onConnect={onConnect}
            isValidConnection={isValidConnection}
            onSelectionChange={({ nodes: sel }) => setSelectedId(sel[0]?.id)}
            fitView
            proOptions={{ hideAttribution: true }}
          >
            <Background
              variant={BackgroundVariant.Dots}
              gap={20}
              size={1}
              color='var(--grid-line)'
            />
            <Controls />
          </ReactFlow>
        </div>
      </div>

      <Card className='scroll flex flex-col gap-4 overflow-y-auto p-3'>
        <NodeInspector
          node={selectedNode}
          onConfigChange={onConfigChange}
          onDelete={onDeleteNode}
          lastRun={inspectorRun}
        />
        <RunOutput result={lastRun} />
      </Card>

      {newFlowDialog}
    </div>
  )
}

function FlowsPlaceholder({
  loading,
  error,
  empty,
}: {
  loading: boolean
  error: unknown
  empty: boolean
}) {
  let message = 'Select a board to render its graph.'
  if (loading) message = 'Loading stored boards…'
  else if (error) message = `Could not load boards: ${String(error)}`
  else if (empty) message = 'No flows yet. Click “New flow” to create one and start adding nodes.'
  return (
    <div className='text-muted-foreground grid h-full place-items-center px-6 text-center text-[12.5px]'>
      {message}
    </div>
  )
}
