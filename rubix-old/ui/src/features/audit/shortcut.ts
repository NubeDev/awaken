/**
 * Classify a keydown event as an undo / redo / no-op shortcut
 * (docs/design/audit-and-undo.md "UI": Cmd/Ctrl+Z undoes, Shift+Cmd/Ctrl+Z
 * redoes). Kept pure (it reads only the event's modifier flags + key) so the
 * mapping is unit-testable without a DOM, and the listener stays a thin shell.
 *
 * A bare key, a key without the platform meta/ctrl modifier, or a key pressed
 * while typing in a field is not a shortcut — the caller skips those so undo
 * never hijacks in-form editing.
 */
export type ShortcutAction = 'undo' | 'redo' | 'none'

/** Modifier + key shape of a keyboard event — the subset the matcher reads. */
export interface ShortcutEvent {
  key: string
  metaKey: boolean
  ctrlKey: boolean
  shiftKey: boolean
}

export function classifyShortcut(e: ShortcutEvent): ShortcutAction {
  const mod = e.metaKey || e.ctrlKey
  if (!mod) return 'none'
  // `z` (and the shifted `Z` some layouts report) is the only relevant key.
  if (e.key.toLowerCase() !== 'z') return 'none'
  return e.shiftKey ? 'redo' : 'undo'
}

/**
 * Whether a keyboard event originates from an editable target (input, textarea,
 * contenteditable, or a CodeMirror surface). The shortcut listener skips these so
 * Cmd-Z keeps its native text-undo inside a field rather than undoing a server
 * change.
 */
export function isEditableTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false
  const tag = target.tagName
  if (tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT') return true
  if (target.isContentEditable) return true
  // CodeMirror editors render an editable .cm-content surface.
  return Boolean(target.closest('.cm-editor'))
}
