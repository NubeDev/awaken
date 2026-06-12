import type { Edge, Node } from '@xyflow/react'
import type { FlowNodeData } from '../components/flow-node'

/**
 * Wiresheet layout for the AHU-3 discharge-reset control board. This is board
 * *structure* (node graph + positions) the canvas renders — not measured data.
 * Stored boards from `/api/v1/boards` will replace this once the board store
 * exposes node geometry; the rendering layer is the same either way.
 */
export const SAMPLE_NODES: Node<FlowNodeData>[] = [
  {
    id: 'n1',
    type: 'block',
    position: { x: 0, y: 40 },
    data: { title: 'Zone Occupancy', sub: 'subscribe', icon: 'route', kind: 'in', hasOut: true },
  },
  {
    id: 'n2',
    type: 'block',
    position: { x: 0, y: 180 },
    data: { title: 'Outside Air Temp', sub: 'subscribe', icon: 'thermometer', kind: 'in', hasOut: true },
  },
  {
    id: 'n3',
    type: 'block',
    position: { x: 260, y: 40 },
    data: { title: 'Schedule', sub: 'occupied → 13°C', icon: 'calendar', kind: 'logic', hasIn: true, hasOut: true },
  },
  {
    id: 'n4',
    type: 'block',
    position: { x: 260, y: 200 },
    data: { title: 'Reset Curve', sub: 'OAT 12–16°C', icon: 'function', kind: 'logic', hasIn: true, hasOut: true },
  },
  {
    id: 'n6',
    type: 'block',
    position: { x: 260, y: 340 },
    data: { title: 'Discharge Temp', sub: 'read_point', icon: 'thermometer', kind: 'in', hasOut: true },
  },
  {
    id: 'n5',
    type: 'block',
    position: { x: 540, y: 180 },
    data: { title: 'PID', sub: 'discharge control', icon: 'route', kind: 'logic', hasIn: true, hasOut: true },
  },
  {
    id: 'n7',
    type: 'block',
    position: { x: 820, y: 100 },
    data: { title: 'Cooling Valve', sub: 'write · prio 13', icon: 'droplet', kind: 'out', hasIn: true },
  },
  {
    id: 'n8',
    type: 'block',
    position: { x: 820, y: 240 },
    data: { title: 'awaken · guard', sub: 'agent_call', icon: 'sparkles', kind: 'agent', hasIn: true, hasOut: true },
  },
]

export const SAMPLE_EDGES: Edge[] = [
  { id: 'e1', source: 'n1', target: 'n3', animated: true },
  { id: 'e2', source: 'n2', target: 'n4', animated: true },
  { id: 'e3', source: 'n3', target: 'n5' },
  { id: 'e4', source: 'n4', target: 'n5' },
  { id: 'e5', source: 'n6', target: 'n5' },
  { id: 'e6', source: 'n5', target: 'n7' },
  { id: 'e7', source: 'n5', target: 'n8' },
]
