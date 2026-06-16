/**
 * The React-Flow node-type registry + a small spec lookup, split out of nodes.tsx
 * so that file only exports components (keeps Vite fast-refresh happy).
 */
import { BlockNode, SourceNode } from './nodes'
import { BLOCK_BY_TYPE, type BlockSpec } from './palette'

export const nodeTypes = {
  block: BlockNode,
  source: SourceNode,
}

/** Resolve a palette block spec by its type, for seeding a node's data. */
export function specFor(type: string): BlockSpec | undefined {
  return BLOCK_BY_TYPE[type]
}
