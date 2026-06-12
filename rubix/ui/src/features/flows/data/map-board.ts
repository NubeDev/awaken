import type { Edge, Node } from '@xyflow/react'
import type { BoardGraph } from '@/api/types'
import type { FlowNodeData } from '../components/flow-node'

/**
 * Map a stored `BoardGraph` onto the React Flow node/edge model the wiresheet
 * renders. The wire carries no canvas geometry (see `api/types.ts`), so node
 * positions are derived deterministically from the graph's wire topology:
 * sources on the left, sinks on the right, one column per dependency depth —
 * the same left-to-right reading the frozen sample look had. The `FlowNode`
 * visuals are untouched; only the data feeding them changed.
 */

/** How a component name presents: its node kind and wiresheet icon. */
type ComponentLook = { kind: FlowNodeData['kind']; icon: string }

/**
 * Registered rubix components (`rubix-flow/src/board/registry.rs`) to their
 * wiresheet look. An unmapped component still renders, falling back to a logic
 * block — honest, not a fabricated label.
 */
const COMPONENT_LOOK: Record<string, ComponentLook> = {
  read_point: { kind: 'in', icon: 'thermometer' },
  query_his: { kind: 'in', icon: 'function' },
  trigger: { kind: 'in', icon: 'calendar' },
  agent_call: { kind: 'agent', icon: 'sparkles' },
  write_point: { kind: 'out', icon: 'droplet' },
  emit_spark: { kind: 'out', icon: 'route' },
}

const FALLBACK_LOOK: ComponentLook = { kind: 'logic', icon: 'route' }

const COL_GAP = 280
const ROW_GAP = 140

/** React Flow nodes + edges for a stored board, laid out by wire depth. */
export function mapBoard(graph: BoardGraph): { nodes: Node<FlowNodeData>[]; edges: Edge[] } {
  const depth = layerDepths(graph)
  const rowCursor = new Map<number, number>()

  const nodes: Node<FlowNodeData>[] = graph.nodes.map((node) => {
    const look = COMPONENT_LOOK[node.component] ?? FALLBACK_LOOK
    const col = depth.get(node.id) ?? 0
    const row = rowCursor.get(col) ?? 0
    rowCursor.set(col, row + 1)
    return {
      id: node.id,
      type: 'block',
      position: { x: col * COL_GAP, y: row * ROW_GAP },
      data: {
        title: node.id,
        sub: node.component,
        icon: look.icon,
        kind: look.kind,
        hasIn: hasInbound(graph, node.id),
        hasOut: hasOutbound(graph, node.id),
      },
    }
  })

  const edges: Edge[] = graph.connections.map((c, i) => ({
    id: `${c.from_node}:${c.from_port}-${c.to_node}:${c.to_port}-${i}`,
    source: c.from_node,
    target: c.to_node,
    animated: true,
  }))

  return { nodes, edges }
}

function hasInbound(graph: BoardGraph, id: string): boolean {
  return graph.connections.some((c) => c.to_node === id)
}

function hasOutbound(graph: BoardGraph, id: string): boolean {
  return graph.connections.some((c) => c.from_node === id)
}

/**
 * Longest-path depth of each node from a source (a node with no inbound wire).
 * Cycles cannot deepen a node past the node count, which bounds the traversal
 * so a malformed graph can never loop forever.
 */
function layerDepths(graph: BoardGraph): Map<string, number> {
  const depth = new Map<string, number>()
  for (const node of graph.nodes) depth.set(node.id, 0)

  const limit = graph.nodes.length
  for (let pass = 0; pass < limit; pass += 1) {
    let changed = false
    for (const c of graph.connections) {
      const next = (depth.get(c.from_node) ?? 0) + 1
      if (next > (depth.get(c.to_node) ?? 0)) {
        depth.set(c.to_node, next)
        changed = true
      }
    }
    if (!changed) break
  }
  return depth
}
