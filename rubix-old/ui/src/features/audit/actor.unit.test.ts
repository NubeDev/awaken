import { describe, expect, it } from 'vitest'
import type { Actor } from '@/api/types'
import { actorKind, actorLabel } from './actor'

describe('actorLabel', () => {
  it('labels a user by subject', () => {
    const a: Actor = { kind: 'user', subject: 'jane@kfc.com' }
    expect(actorLabel(a)).toBe('jane@kfc.com')
  })

  it('labels an agent with its model', () => {
    const a: Actor = { kind: 'agent', run_id: 'r1', model: 'opus-4' }
    expect(actorLabel(a)).toBe('Agent (opus-4)')
  })

  it('labels the system actor', () => {
    expect(actorLabel({ kind: 'system' })).toBe('System')
  })
})

describe('actorKind', () => {
  it('returns the discriminant', () => {
    expect(actorKind({ kind: 'agent', run_id: 'r', model: 'm' })).toBe('agent')
  })
})
