/**
 * A toast with an inline "Undo" action shown after a recorded mutation
 * (docs/design/audit-and-undo.md "UI": 'Deleted dashboard · Undo'). The toast's
 * action fires the same per-actor `POST /undo` the toolbar/shortcut use, so the
 * server pops the caller's newest change group — which is the one just made — and
 * the returned touched ids drive the canvas refresh.
 *
 * The undo is server-driven by the actor cursor, so the toast does not need to
 * carry the change rows; it only needs the org to scope the cursor. The optional
 * `group` is surfaced for tracing/debugging and so a future targeted undo can use
 * it without a signature change.
 */
import { toast } from 'sonner'
import { useUndo } from '@/api/hooks'
import { useScope } from '@/context/scope-provider'

interface UndoToastOptions {
  /** Verb-phrase headline, e.g. 'Deleted dashboard'. */
  message: string
  /** The change group id this mutation produced (for tracing). */
  group?: string
}

export function useUndoToast() {
  const { org } = useScope()
  const undo = useUndo(org)

  return (opts: UndoToastOptions) => {
    toast(opts.message, {
      description: opts.group ? `Change ${opts.group.slice(0, 8)}` : undefined,
      action: {
        label: 'Undo',
        onClick: () => {
          if (!org) return
          undo.mutate(undefined, {
            onSuccess: () => toast.success('Change undone'),
            onError: (e) =>
              toast.error('Undo failed', {
                description: (e as Error).message,
              }),
          })
        },
      },
    })
  }
}
