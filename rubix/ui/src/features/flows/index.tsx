import { useCallback, useMemo, useState } from 'react'
import {
  Background,
  BackgroundVariant,
  Controls,
  ReactFlow,
  useEdgesState,
  useNodesState,
  type Connection,
  type Node,
  addEdge,
} from '@xyflow/react'
import '@xyflow/react/dist/style.css'
import { Main } from '@/components/layout/main'
import { PageHeader } from '@/components/layout/page-header'
import { Card } from '@/components/ui/card'
import { BoardStatusBar } from './components/board-status-bar'
import { FlowNode, type FlowNodeData } from './components/flow-node'
import { NodeInspector } from './components/node-inspector'
import { NodePalette } from './components/node-palette'
import { SAMPLE_EDGES, SAMPLE_NODES } from './data/sample-board'

export function Flows() {
  const nodeTypes = useMemo(() => ({ block: FlowNode }), [])
  const [nodes, , onNodesChange] = useNodesState(SAMPLE_NODES)
  const [edges, setEdges, onEdgesChange] = useEdgesState(SAMPLE_EDGES)
  const [selected, setSelected] = useState<Node<FlowNodeData> | undefined>()
  const onConnect = useCallback(
    (c: Connection) => setEdges((eds) => addEdge(c, eds)),
    [setEdges]
  )

  return (
    <>
      <PageHeader title='Flow Boards' sub='reflow control & analytics graphs' />
      <Main fluid fixed className='flex min-h-0'>
        <div className='grid min-h-0 w-full flex-1 gap-3 lg:grid-cols-[200px_1fr_230px]'>
          <Card className='scroll overflow-y-auto p-2.5'>
            <NodePalette />
          </Card>

          <div className='flex min-h-0 flex-col'>
            <BoardStatusBar
              name='AHU-3 · Discharge Reset'
              nodeCount={nodes.length}
              edgeCount={edges.length}
            />
            <div className='border-border min-h-0 flex-1 overflow-hidden rounded-lg border'>
              <ReactFlow
                nodes={nodes}
                edges={edges}
                nodeTypes={nodeTypes}
                onNodesChange={onNodesChange}
                onEdgesChange={onEdgesChange}
                onConnect={onConnect}
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
            </div>
          </div>

          <Card className='scroll overflow-y-auto p-3'>
            <NodeInspector node={selected} />
          </Card>
        </div>
      </Main>
    </>
  )
}
