/**
 * The undo/redo toolbar control + global keyboard shortcuts
 * (docs/design/audit-and-undo.md "UI"). Cmd/Ctrl+Z undoes and Shift+Cmd/Ctrl+Z
 * redoes the authenticated principal's own most-recent change group; on success
 * the returned touched ids invalidate exactly the matching queries (handled in
 * `useUndo`/`useRedo`) so the canvas refreshes without a reload.
 *
 * Undo is per-actor and server-driven, so the buttons are always available when
 * an org is in scope; the server returns an empty `touched` (and we surface a
 * "nothing to undo" toast) when the cursor is at an end.
 */
import { useEffect } from 'react'
import { Redo2, Undo2 } from 'lucide-react'
import { toast } from 'sonner'
import { useRedo, useUndo } from '@/api/hooks'
import type { UndoResult } from '@/api/types'
import { useScope } from '@/context/scope-provider'
import { Button } from '@/components/ui/button'
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from '@/components/ui/tooltip'
import { classifyShortcut, isEditableTarget } from './shortcut'

export function UndoRedoControl() {
  const { org } = useScope()
  const undo = useUndo(org)
  const redo = useRedo(org)

  const report = (verb: 'Undid' | 'Redid') => (result: UndoResult) => {
    if (!result.group) {
      toast(verb === 'Undid' ? 'Nothing to undo' : 'Nothing to redo')
      return
    }
    toast.success(`${verb} change`)
  }
  const onError = (verb: string) => (e: unknown) =>
    toast.error(`${verb} failed`, { description: (e as Error).message })

  const runUndo = () => {
    if (!org || undo.isPending) return
    undo.mutate(undefined, {
      onSuccess: report('Undid'),
      onError: onError('Undo'),
    })
  }
  const runRedo = () => {
    if (!org || redo.isPending) return
    redo.mutate(undefined, {
      onSuccess: report('Redid'),
      onError: onError('Redo'),
    })
  }

  useEffect(() => {
    const down = (e: KeyboardEvent) => {
      if (isEditableTarget(e.target)) return
      const action = classifyShortcut(e)
      if (action === 'none') return
      e.preventDefault()
      if (action === 'undo') runUndo()
      else runRedo()
    }
    document.addEventListener('keydown', down)
    return () => document.removeEventListener('keydown', down)
    // `runUndo`/`runRedo` close over the current org + mutation handles, which are
    // stable for a given scope; re-binding on org change keeps the cursor correct.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [org])

  if (!org) return null

  return (
    <div className='flex items-center'>
      <Tooltip>
        <TooltipTrigger asChild>
          <Button
            size='icon'
            variant='ghost'
            className='size-8'
            aria-label='Undo'
            disabled={undo.isPending}
            onClick={runUndo}
          >
            <Undo2 className='size-4' />
          </Button>
        </TooltipTrigger>
        <TooltipContent>Undo (⌘Z)</TooltipContent>
      </Tooltip>
      <Tooltip>
        <TooltipTrigger asChild>
          <Button
            size='icon'
            variant='ghost'
            className='size-8'
            aria-label='Redo'
            disabled={redo.isPending}
            onClick={runRedo}
          >
            <Redo2 className='size-4' />
          </Button>
        </TooltipTrigger>
        <TooltipContent>Redo (⇧⌘Z)</TooltipContent>
      </Tooltip>
    </div>
  )
}
