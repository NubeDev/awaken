/**
 * The wiresheet canvas — a React-Flow editor where the user wires the kit blocks
 * (palette.ts) and live source points (source-nodes.ts) into extra EMS logic.
 * Drop from the palette to add a node; drag handle-to-handle to connect. The graph
 * is local component state only (this is a demo surface — nothing is persisted or
 * evaluated). A starter "high-voltage alarm" sheet is seeded so the canvas opens
 * with something to read.
 *
 * Edges are animated and colour-inherited from their source port so signal flow
 * reads like a real wiresheet. The selected node drives the right-hand inspector
 * (wiresheet-page owns that — we lift selection up via onSelect).
 */
import { useCallback, useEffect, useRef, useState } from 'react'
import {
  Background,
  BackgroundVariant,
  Controls,
  MiniMap,
  ReactFlow,
  addEdge,
  reconnectEdge,
  useEdgesState,
  useNodesState,
  type Connection,
  type Edge,
  type Node,
  type ReactFlowInstance,
} from '@xyflow/react'
import '@xyflow/react/dist/style.css'
import { nodeTypes, specFor } from './node-types'
import { DND_MIME, type DragPayload } from './palette-panel'
import { accentVar, portColor, type BlockSpec } from './palette'
import type { SourcePoint } from './source-nodes'

let seq = 100
const nextId = () => `n${seq++}`

/** A source point keyed by its node `type` (`src:<registerId>`), for drop lookup. */
export type SourceLookup = Map<string, SourcePoint>

function makeBlockNode(type: string, position: { x: number; y: number }): Node | null {
  const spec = specFor(type)
  if (!spec) return null
  return {
    id: nextId(),
    type: 'block',
    position,
    data: { spec },
  }
}

function makeSourceNode(
  point: SourcePoint,
  position: { x: number; y: number }
): Node {
  return {
    id: nextId(),
    type: 'source',
    position,
    data: { point },
  }
}

const edgeStyle = (color: string) => ({
  stroke: color,
  strokeWidth: 2,
})

/** Seed a small, legible "voltage over 253 V → email + console alarm" sheet. */
function seedGraph(sources: SourceLookup): { nodes: Node[]; edges: Edge[] } {
  // Pick the first voltage source if one exists, else any source.
  const points = [...sources.values()]
  const volt =
    points.find((p) => p.quantity === 'voltage') ?? points[0] ?? null

  const nodes: Node[] = []
  const edges: Edge[] = []

  if (volt) {
    nodes.push(makeSourceNode(volt, { x: 40, y: 160 }))
  }
  const avg = makeBlockNode('average', { x: 320, y: 150 })!
  const thr = makeBlockNode('threshold-alarm', { x: 580, y: 140 })!
  const delay = makeBlockNode('delay', { x: 820, y: 130 })!
  const email = makeBlockNode('notify-email', { x: 1060, y: 80 })!
  const bell = makeBlockNode('alarm-bell', { x: 1060, y: 220 })!
  nodes.push(avg, thr, delay, email, bell)

  const connect = (
    source: string,
    sourceHandle: string,
    target: string,
    targetHandle: string,
    color: string
  ) =>
    edges.push({
      id: `e${source}-${target}`,
      source,
      sourceHandle,
      target,
      targetHandle,
      animated: true,
      style: edgeStyle(color),
    })

  if (volt) connect(nodes[0].id, 'out', avg.id, 'in', portColor('number'))
  connect(avg.id, 'out', thr.id, 'in', portColor('number'))
  connect(thr.id, 'sev', delay.id, 'in', portColor('event'))
  connect(delay.id, 'out', email.id, 'trig', portColor('bool'))
  connect(delay.id, 'out', bell.id, 'trig', portColor('bool'))

  return { nodes, edges }
}

export interface CanvasHandle {
  clear: () => void
  /** Write a node's (demo) config value back so the canvas re-renders it. */
  setConfig: (nodeId: string, value: string) => void
}

export function WiresheetCanvas({
  sources,
  onSelect,
  registerHandle,
}: {
  sources: SourceLookup
  onSelect: (node: Node | null) => void
  registerHandle?: (h: CanvasHandle) => void
}) {
  // Seed the starter sheet exactly once, lazily, from the sources available on
  // first mount (a lazy useState initialiser runs only on the initial render).
  const [seed] = useState(() => seedGraph(sources))
  const [nodes, setNodes, onNodesChange] = useNodesState(seed.nodes)
  const [edges, setEdges, onEdgesChange] = useEdgesState(seed.edges)
  const flowRef = useRef<ReactFlowInstance | null>(null)
  const edgeReconnectOk = useRef(true)

  // Colour a new edge from the source-handle's port kind (best-effort: blocks
  // expose typed ports; default to analog).
  const onConnect = useCallback(
    (c: Connection) => {
      const srcNode = nodes.find((n) => n.id === c.source)
      let color = portColor('number')
      if (srcNode?.type === 'block') {
        const spec = specFor((srcNode.data as { spec: { type: string } }).spec.type)
        const port = spec?.outputs.find((p) => p.id === c.sourceHandle)
        if (port) color = portColor(port.kind)
      } else if (srcNode?.type === 'source') {
        color = portColor('number')
      }
      setEdges((eds) =>
        addEdge(
          { ...c, animated: true, style: edgeStyle(color) },
          eds
        )
      )
    },
    [nodes, setEdges]
  )

  // Allow dragging an edge end to a new port; drop in empty space deletes it.
  const onReconnectStart = useCallback(() => {
    edgeReconnectOk.current = false
  }, [])
  const onReconnect = useCallback(
    (oldEdge: Edge, newConn: Connection) => {
      edgeReconnectOk.current = true
      setEdges((els) => reconnectEdge(oldEdge, newConn, els))
    },
    [setEdges]
  )
  const onReconnectEnd = useCallback(
    (_: unknown, edge: Edge) => {
      if (!edgeReconnectOk.current) {
        setEdges((eds) => eds.filter((e) => e.id !== edge.id))
      }
      edgeReconnectOk.current = true
    },
    [setEdges]
  )

  const onDragOver = useCallback((e: React.DragEvent) => {
    e.preventDefault()
    e.dataTransfer.dropEffect = 'move'
  }, [])

  const onDrop = useCallback(
    (e: React.DragEvent) => {
      e.preventDefault()
      const raw = e.dataTransfer.getData(DND_MIME)
      if (!raw || !flowRef.current) return
      const payload = JSON.parse(raw) as DragPayload
      const position = flowRef.current.screenToFlowPosition({
        x: e.clientX,
        y: e.clientY,
      })

      let node: Node | null = null
      if (payload.kind === 'block') {
        node = makeBlockNode(payload.type, position)
      } else {
        const point = sources.get(payload.type)
        if (point) node = makeSourceNode(point, position)
      }
      if (node) setNodes((nds) => nds.concat(node!))
    },
    [setNodes, sources]
  )

  useEffect(() => {
    if (!registerHandle) return
    registerHandle({
      clear: () => {
        setNodes([])
        setEdges([])
        onSelect(null)
      },
      setConfig: (nodeId, value) => {
        setNodes((nds) =>
          nds.map((n) => {
            if (n.id !== nodeId) return n
            const spec = (n.data as { spec?: BlockSpec }).spec
            if (!spec?.config) return n
            return {
              ...n,
              data: {
                ...n.data,
                spec: { ...spec, config: { ...spec.config, value } },
              },
            }
          })
        )
      },
    })
  }, [registerHandle, setNodes, setEdges, onSelect])

  return (
    <ReactFlow
      nodes={nodes}
      edges={edges}
      nodeTypes={nodeTypes}
      onNodesChange={onNodesChange}
      onEdgesChange={onEdgesChange}
      onConnect={onConnect}
      onReconnect={onReconnect}
      onReconnectStart={onReconnectStart}
      onReconnectEnd={onReconnectEnd}
      onInit={(inst) => {
        flowRef.current = inst
      }}
      onDrop={onDrop}
      onDragOver={onDragOver}
      onSelectionChange={({ nodes: sel }) => onSelect(sel[0] ?? null)}
      fitView
      fitViewOptions={{ padding: 0.25, maxZoom: 1.1 }}
      defaultEdgeOptions={{ animated: true }}
      proOptions={{ hideAttribution: true }}
      minZoom={0.3}
      maxZoom={2}
      className='bg-transparent'
    >
      <Background
        variant={BackgroundVariant.Dots}
        gap={18}
        size={1}
        className='!bg-muted/20'
        color='var(--border)'
      />
      <MiniMap
        pannable
        zoomable
        className='!bg-card/80 !border !rounded-lg !shadow-sm'
        maskColor='color-mix(in oklch, var(--muted) 60%, transparent)'
        nodeColor={(n) =>
          n.type === 'source'
            ? accentVar(2)
            : accentVar(
                (specFor((n.data as { spec?: { type: string } }).spec?.type ?? '')
                  ?.accent ?? 2)
              )
        }
        nodeStrokeWidth={2}
      />
      <Controls
        className='!bg-card/80 !border !rounded-lg !shadow-sm [&_button]:!border-border [&_button]:!bg-transparent [&_button:hover]:!bg-accent'
        showInteractive={false}
      />
    </ReactFlow>
  )
}
