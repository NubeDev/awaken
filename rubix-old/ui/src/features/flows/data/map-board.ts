import type { Edge, Node } from '@xyflow/react'
import type { BoardConnection, BoardGraph, ComponentView, PortType } from '@/api/types'
import type { FlowNodeData } from '../components/flow-node'

/**
 * Bridge between the stored `BoardGraph` and the React Flow node/edge model the
 * editor mutates. The wire carries no canvas geometry, so on load node
 * positions are derived deterministically from wire topology (sources left,
 * sinks right, one column per dependency depth). Each node binds to its
 * `ComponentView` schema so the canvas can draw typed handles and the inspector
 * can render a config form. `graphFromFlow` is the inverse, serializing the
 * edited canvas back to a `BoardGraph` for save.
 */

const COL_GAP = 280
const ROW_GAP = 150

/** A lookup from component name to its schema, for binding nodes. */
export type ComponentIndex = Map<string, ComponentView>

export function indexComponents(components: ComponentView[]): ComponentIndex {
  return new Map(components.map((c) => [c.component, c]))
}

/** React Flow nodes + edges for a stored board, laid out by wire depth. */
export function mapBoard(
  graph: BoardGraph,
  components: ComponentIndex
): { nodes: Node<FlowNodeData>[]; edges: Edge[] } {
  const depth = layerDepths(graph)
  const rowCursor = new Map<number, number>()

  const nodes: Node<FlowNodeData>[] = graph.nodes.flatMap((node) => {
    const schema = components.get(node.component)
    // An unknown component cannot be drawn (no port schema) — drop it rather
    // than fabricate ports. The save round-trip preserves only what renders.
    if (!schema) return []
    const col = depth.get(node.id) ?? 0
    const row = rowCursor.get(col) ?? 0
    rowCursor.set(col, row + 1)
    return [
      {
        id: node.id,
        type: 'block',
        position: { x: col * COL_GAP, y: row * ROW_GAP },
        data: {
          nodeId: node.id,
          component: node.component,
          config: { ...node.config },
          schema,
        },
      },
    ]
  })

  const edges: Edge[] = graph.connections.map(edgeFromConnection)
  return { nodes, edges }
}

/**
 * Whether a `source` port type may feed a `target` inport — mirrors
 * `PortType::accepts` in `rubix-flow`. A flow tick drives any inport; otherwise
 * the data classes must match. Kept in lockstep with the backend so the editor
 * never offers a wire the engine would reject.
 */
export function portAccepts(source: PortType, target: PortType): boolean {
  if (source === 'flow') return true
  return source === target
}

/** The `port_type` of a node's outport / inport, or undefined if unknown. */
export function outportType(
  node: Node<FlowNodeData> | undefined,
  portId: string | null | undefined
): PortType | undefined {
  return node?.data.schema.outports.find((p) => p.id === portId)?.port_type
}

export function inportType(
  node: Node<FlowNodeData> | undefined,
  portId: string | null | undefined
): PortType | undefined {
  return node?.data.schema.inports.find((p) => p.id === portId)?.port_type
}

/** Stable edge id for a connection, so React Flow can dedupe and update it. */
export function edgeId(c: BoardConnection): string {
  return `${c.from_node}:${c.from_port}->${c.to_node}:${c.to_port}`
}

function edgeFromConnection(c: BoardConnection): Edge {
  return {
    id: edgeId(c),
    source: c.from_node,
    sourceHandle: c.from_port,
    target: c.to_node,
    targetHandle: c.to_port,
    animated: true,
  }
}

/**
 * Serialize the edited canvas back to a `BoardGraph`. Node config and ids come
 * straight off the canvas; an edge maps to a `BoardConnection` via its handle
 * ids (which are the port names). Canvas geometry is intentionally dropped — it
 * is not part of the wire format.
 */
export function graphFromFlow(nodes: Node<FlowNodeData>[], edges: Edge[]): BoardGraph {
  return {
    nodes: nodes.map((n) => ({
      id: n.data.nodeId,
      component: n.data.component,
      config: n.data.config,
    })),
    connections: edges.flatMap((e) => {
      // A connection needs both port names; handles always carry them for our
      // multi-port nodes, but guard so a malformed edge is dropped, not saved
      // with empty ports.
      if (!e.sourceHandle || !e.targetHandle) return []
      return [
        {
          from_node: e.source,
          from_port: e.sourceHandle,
          to_node: e.target,
          to_port: e.targetHandle,
        },
      ]
    }),
  }
}

/**
 * Longest-path depth of each node from a source (a node with no inbound wire).
 * Bounded by the node count so a malformed (cyclic) graph cannot loop forever.
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
