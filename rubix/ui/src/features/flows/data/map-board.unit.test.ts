import { describe, expect, it } from 'vitest'
import type { BoardGraph } from '@/api/types'
import { mapBoard } from './map-board'

const graph: BoardGraph = {
  nodes: [
    { id: 'read-occupancy', component: 'read_point', config: {} },
    { id: 'agent-guard', component: 'agent_call', config: {} },
    { id: 'write-cooling-valve', component: 'write_point', config: {} },
  ],
  connections: [
    { from_node: 'read-occupancy', from_port: 'output', to_node: 'agent-guard', to_port: 'value' },
    { from_node: 'agent-guard', from_port: 'output', to_node: 'write-cooling-valve', to_port: 'value' },
  ],
}

describe('mapBoard', () => {
  it('maps every node and connection', () => {
    const { nodes, edges } = mapBoard(graph)
    expect(nodes).toHaveLength(3)
    expect(edges).toHaveLength(2)
  })

  it('lays sources left and sinks right by wire depth', () => {
    const { nodes } = mapBoard(graph)
    const x = (id: string) => nodes.find((n) => n.id === id)!.position.x
    expect(x('read-occupancy')).toBeLessThan(x('agent-guard'))
    expect(x('agent-guard')).toBeLessThan(x('write-cooling-valve'))
  })

  it('derives kind/icon and handle flags from the component and topology', () => {
    const { nodes } = mapBoard(graph)
    const read = nodes.find((n) => n.id === 'read-occupancy')!
    expect(read.data.kind).toBe('in')
    expect(read.data.hasIn).toBe(false)
    expect(read.data.hasOut).toBe(true)
    const agent = nodes.find((n) => n.id === 'agent-guard')!
    expect(agent.data.kind).toBe('agent')
    expect(agent.data.hasIn).toBe(true)
    expect(agent.data.hasOut).toBe(true)
    const write = nodes.find((n) => n.id === 'write-cooling-valve')!
    expect(write.data.kind).toBe('out')
    expect(write.data.hasOut).toBe(false)
  })

  it('falls back to a logic block for an unmapped component without looping', () => {
    const cyclic: BoardGraph = {
      nodes: [
        { id: 'a', component: 'mystery', config: {} },
        { id: 'b', component: 'read_point', config: {} },
      ],
      connections: [
        { from_node: 'a', from_port: 'o', to_node: 'b', to_port: 'i' },
        { from_node: 'b', from_port: 'o', to_node: 'a', to_port: 'i' },
      ],
    }
    const { nodes } = mapBoard(cyclic)
    expect(nodes.find((n) => n.id === 'a')!.data.kind).toBe('logic')
  })
})
