/**
 * Human-readable label for a change `Actor` (docs/design/audit-and-undo.md: an
 * actor is a user, the AI agent runtime, or the system/scheduler). Shared by the
 * History tab and the admin Audit screen so both render attribution identically.
 * Pure, so it is unit-testable without rendering.
 */
import type { Actor } from '@/api/types'

export function actorLabel(actor: Actor): string {
  switch (actor.kind) {
    case 'user':
      return actor.subject
    case 'agent':
      return `Agent (${actor.model})`
    case 'system':
      return 'System'
  }
}

/** A short kind tag for an actor badge ('user' | 'agent' | 'system'). */
export function actorKind(actor: Actor): Actor['kind'] {
  return actor.kind
}
