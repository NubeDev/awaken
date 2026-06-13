import { describe, expect, it } from 'vitest'
import { classifyShortcut, type ShortcutEvent } from './shortcut'

const ev = (over: Partial<ShortcutEvent>): ShortcutEvent => ({
  key: 'z',
  metaKey: false,
  ctrlKey: false,
  shiftKey: false,
  ...over,
})

describe('classifyShortcut', () => {
  it('Cmd+Z is undo', () => {
    expect(classifyShortcut(ev({ metaKey: true }))).toBe('undo')
  })

  it('Ctrl+Z is undo (non-mac)', () => {
    expect(classifyShortcut(ev({ ctrlKey: true }))).toBe('undo')
  })

  it('Shift+Cmd+Z is redo', () => {
    expect(classifyShortcut(ev({ metaKey: true, shiftKey: true }))).toBe('redo')
  })

  it('a bare z is no shortcut', () => {
    expect(classifyShortcut(ev({}))).toBe('none')
  })

  it('a non-z key with the modifier is no shortcut', () => {
    expect(classifyShortcut(ev({ key: 'k', metaKey: true }))).toBe('none')
  })

  it('an uppercase Z (shifted layout) still classifies', () => {
    expect(classifyShortcut(ev({ key: 'Z', metaKey: true, shiftKey: true }))).toBe(
      'redo'
    )
  })
})
