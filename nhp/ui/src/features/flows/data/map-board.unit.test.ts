import { describe, expect, it } from 'vitest'
import type { BoardGraph, ComponentView, PortType } from '@/api/types'
import { graphFromFlow, indexComponents, mapBoard, portAccepts } from './map-board'

type Port = [id: string, port_type: PortType]

function comp(
  component: string,
  kind: ComponentView['kind'],
  inports: Port[],
  outports: Port[]
): ComponentView {
  return {
    component,
    label: component,
    description: '',
    kind,
    inports: inports.map(([id, port_type]) => ({ id, label: id, port_type })),
    outports: outports.map(([id, port_type]) => ({ id, label: id, port_type })),
    config: [],
  }
}

const components = indexComponents([
  comp('read_point', 'source', [['trigger', 'flow']], [['output', 'scalar'], ['error', 'error']]),
  comp(
    'agent_call',
    'agent',
    [['value', 'scalar']],
    [['output', 'scalar'], ['error', 'error']]
  ),
  comp('write_point', 'sink', [['value', 'scalar']], [['output', 'scalar'], ['error', 'error']]),
])

const graph: BoardGraph = {
  nodes: [
    { id: 'read-occupancy', component: 'read_point', config: { point: 'nube/hq/occ' } },
    { id: 'agent-guard', component: 'agent_call', config: {} },
    { id: 'write-cooling-valve', component: 'write_point', config: { priority: 13 } },
  ],
  connections: [
    { from_node: 'read-occupancy', from_port: 'output', to_node: 'agent-guard', to_port: 'value' },
    {
      from_node: 'agent-guard',
      from_port: 'output',
      to_node: 'write-cooling-valve',
      to_port: 'value',
    },
  ],
}

describe('mapBoard', () => {
  it('maps every node and connection, binding each node to its schema', () => {
    const { nodes, edges } = mapBoard(graph, components)
    expect(nodes).toHaveLength(3)
    expect(edges).toHaveLength(2)
    const read = nodes.find((n) => n.id === 'read-occupancy')!
    expect(read.data.schema.kind).toBe('source')
    expect(read.data.config).toEqual({ point: 'nube/hq/occ' })
  })

  it('carries port names on edges as React Flow handle ids', () => {
    const { edges } = mapBoard(graph, components)
    const first = edges[0]
    expect(first.sourceHandle).toBe('output')
    expect(first.targetHandle).toBe('value')
  })

  it('lays sources left and sinks right by wire depth', () => {
    const { nodes } = mapBoard(graph, components)
    const x = (id: string) => nodes.find((n) => n.id === id)!.position.x
    expect(x('read-occupancy')).toBeLessThan(x('agent-guard'))
    expect(x('agent-guard')).toBeLessThan(x('write-cooling-valve'))
  })

  it('drops an unknown component instead of fabricating ports, without looping', () => {
    const cyclic: BoardGraph = {
      nodes: [
        { id: 'a', component: 'mystery', config: {} },
        { id: 'b', component: 'read_point', config: {} },
      ],
      connections: [
        { from_node: 'a', from_port: 'o', to_node: 'b', to_port: 'trigger' },
        { from_node: 'b', from_port: 'output', to_node: 'a', to_port: 'i' },
      ],
    }
    const { nodes } = mapBoard(cyclic, components)
    expect(nodes.map((n) => n.id)).toEqual(['b'])
  })

  it('round-trips through graphFromFlow preserving nodes, config, and wires', () => {
    const { nodes, edges } = mapBoard(graph, components)
    const out = graphFromFlow(nodes, edges)
    expect(out.nodes).toEqual(graph.nodes)
    expect(out.connections).toEqual(graph.connections)
  })
})

describe('portAccepts', () => {
  it('lets a flow tick drive any inport', () => {
    expect(portAccepts('flow', 'scalar')).toBe(true)
    expect(portAccepts('flow', 'object')).toBe(true)
  })

  it('requires matching data classes', () => {
    expect(portAccepts('scalar', 'scalar')).toBe(true)
    expect(portAccepts('scalar', 'object')).toBe(false)
    expect(portAccepts('object', 'scalar')).toBe(false)
  })
})
