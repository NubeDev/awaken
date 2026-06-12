import { useCallback, useMemo, useState } from 'react'
import {
  Background,
  BackgroundVariant,
  Controls,
  ReactFlow,
  type Node,
} from '@xyflow/react'
import '@xyflow/react/dist/style.css'
import { toast } from 'sonner'
import { useBoards, useRunStoredBoard } from '@/api/hooks'
import type { RunBoardResponse } from '@/api/types'
import { Main } from '@/components/layout/main'
import { PageHeader } from '@/components/layout/page-header'
import { Card } from '@/components/ui/card'
import { BoardPicker } from './components/board-picker'
import { BoardStatusBar } from './components/board-status-bar'
import { FlowNode, type FlowNodeData } from './components/flow-node'
import { NodeInspector } from './components/node-inspector'
import { NodePalette } from './components/node-palette'
import { RunOutput } from './components/run-output'
import { mapBoard } from './data/map-board'

export function Flows() {
  const nodeTypes = useMemo(() => ({ block: FlowNode }), [])
  const boardsQuery = useBoards()
  const runBoard = useRunStoredBoard()
  const boards = useMemo(() => boardsQuery.data ?? [], [boardsQuery.data])

  // No explicit selection yet means "the first board"; deriving it (rather than
  // syncing via an effect) keeps the picker honest before the list resolves.
  const [pickedSlug, setPickedSlug] = useState<string | undefined>()
  const selectedSlug = pickedSlug ?? boards[0]?.slug
  const board = useMemo(
    () => boards.find((b) => b.slug === selectedSlug),
    [boards, selectedSlug]
  )

  const [selected, setSelected] = useState<Node<FlowNodeData> | undefined>()
  const [lastRun, setLastRun] = useState<RunBoardResponse | undefined>()

  // Deterministic layout for the current board's graph (the wire carries no
  // geometry). Keying the canvas on the slug below resets canvas state on swap.
  const { nodes, edges } = useMemo(
    () => (board ? mapBoard(board.graph) : { nodes: [], edges: [] }),
    [board]
  )

  const onSelectBoard = useCallback((slug: string) => {
    setPickedSlug(slug)
    setSelected(undefined)
    setLastRun(undefined)
  }, [])

  const onTestRun = useCallback(() => {
    if (!board) return
    runBoard.mutate(board.slug, {
      onSuccess: (res) => {
        setLastRun(res)
        toast('Board run complete', {
          description: `${res.outputs.length} outport packet${res.outputs.length === 1 ? '' : 's'} produced`,
        })
      },
      onError: (err) => toast.error('Board run failed', { description: String(err) }),
    })
  }, [board, runBoard])

  return (
    <>
      <PageHeader title='Flow Boards' sub='reflow control & analytics graphs' />
      <Main fluid fixed className='flex min-h-0'>
        <div className='grid min-h-0 w-full flex-1 gap-3 lg:grid-cols-[200px_1fr_230px]'>
          <Card className='scroll overflow-y-auto p-2.5'>
            <NodePalette />
          </Card>

          <div className='flex min-h-0 flex-col'>
            <div className='pb-2.5'>
              <BoardPicker
                boards={boards}
                selectedSlug={selectedSlug}
                onSelect={onSelectBoard}
              />
            </div>
            {board && (
              <BoardStatusBar
                name={board.display_name}
                version={board.version}
                enabled={board.enabled}
                nodeCount={board.graph.nodes.length}
                edgeCount={board.graph.connections.length}
                running={runBoard.isPending}
                onTestRun={onTestRun}
              />
            )}
            <div className='border-border min-h-0 flex-1 overflow-hidden rounded-lg border'>
              {board ? (
                <ReactFlow
                  key={board.slug}
                  defaultNodes={nodes}
                  defaultEdges={edges}
                  nodeTypes={nodeTypes}
                  onSelectionChange={({ nodes: sel }) =>
                    setSelected(sel[0] as Node<FlowNodeData> | undefined)
                  }
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
              ) : (
                <FlowsPlaceholder
                  loading={boardsQuery.isLoading}
                  error={boardsQuery.error}
                  empty={boards.length === 0}
                />
              )}
            </div>
          </div>

          <Card className='scroll flex flex-col gap-4 overflow-y-auto p-3'>
            <NodeInspector node={selected} />
            <RunOutput result={lastRun} />
          </Card>
        </div>
      </Main>
    </>
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
  else if (empty) message = 'No stored boards yet. Seed the dev portfolio or publish a board.'
  return (
    <div className='text-muted-foreground grid h-full place-items-center px-6 text-center text-[12.5px]'>
      {message}
    </div>
  )
}
