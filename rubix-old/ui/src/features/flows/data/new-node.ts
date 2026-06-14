import type { Node } from '@xyflow/react'
import type { ComponentView } from '@/api/types'
import type { FlowNodeData } from '../components/flow-node'

/**
 * A fresh node for a dropped component. Required fields seed from their schema
 * default (so a new node is valid where possible); optional fields stay unset
 * and fall back to the actor default at run time. The id is unique within the
 * current node set so the graph stays addressable.
 */
export function newNode(
  schema: ComponentView,
  position: { x: number; y: number },
  existingIds: Set<string>
): Node<FlowNodeData> {
  const id = uniqueId(schema.component, existingIds)
  const config: Record<string, unknown> = {}
  for (const field of schema.config) {
    if (field.required && field.default !== undefined) {
      config[field.name] = field.default
    }
  }
  return {
    id,
    type: 'block',
    position,
    data: { nodeId: id, component: schema.component, config, schema },
  }
}

/** `component`, `component-2`, … — first form not already taken. */
function uniqueId(base: string, existing: Set<string>): string {
  if (!existing.has(base)) return base
  for (let n = 2; ; n += 1) {
    const candidate = `${base}-${n}`
    if (!existing.has(candidate)) return candidate
  }
}
