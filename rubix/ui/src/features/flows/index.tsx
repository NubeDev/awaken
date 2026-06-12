import { useCallback, useMemo } from 'react'
import {
  Background,
  BackgroundVariant,
  Controls,
  ReactFlow,
  useEdgesState,
  useNodesState,
  type Connection,
  addEdge,
} from '@xyflow/react'
import '@xyflow/react/dist/style.css'
import { ConfigDrawer } from '@/components/config-drawer'
import { Header } from '@/components/layout/header'
import { Main } from '@/components/layout/main'
import { ProfileDropdown } from '@/components/profile-dropdown'
import { Search } from '@/components/search'
import { ThemeSwitch } from '@/components/theme-switch'
import { FlowNode } from './components/flow-node'
import { SAMPLE_EDGES, SAMPLE_NODES } from './data/sample-board'

export function Flows() {
  const nodeTypes = useMemo(() => ({ block: FlowNode }), [])
  const [nodes, , onNodesChange] = useNodesState(SAMPLE_NODES)
  const [edges, setEdges, onEdgesChange] = useEdgesState(SAMPLE_EDGES)
  const onConnect = useCallback(
    (c: Connection) => setEdges((eds) => addEdge(c, eds)),
    [setEdges]
  )

  return (
    <>
      <Header>
        <Search />
        <div className='ms-auto flex items-center gap-2'>
          <ThemeSwitch />
          <ConfigDrawer />
          <ProfileDropdown />
        </div>
      </Header>
      <Main fixed className='flex flex-col'>
        <div className='mb-3'>
          <h1 className='text-2xl font-bold tracking-tight'>Flow Boards</h1>
          <p className='text-muted-foreground text-sm'>AHU-3 · Discharge Reset</p>
        </div>
        <div className='min-h-0 flex-1 overflow-hidden rounded-lg border border-border'>
          <ReactFlow
            nodes={nodes}
            edges={edges}
            nodeTypes={nodeTypes}
            onNodesChange={onNodesChange}
            onEdgesChange={onEdgesChange}
            onConnect={onConnect}
            fitView
            proOptions={{ hideAttribution: true }}
          >
            <Background variant={BackgroundVariant.Dots} gap={20} size={1} color='var(--grid-line)' />
            <Controls />
          </ReactFlow>
        </div>
      </Main>
    </>
  )
}
