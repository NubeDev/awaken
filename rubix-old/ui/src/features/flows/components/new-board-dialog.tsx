import { useState } from 'react'
import { toast } from 'sonner'
import { useSaveBoard } from '@/api/hooks'
import { useScope } from '@/context/scope-provider'
import type { BoardView, Trigger } from '@/api/types'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'

type NewBoardDialogProps = {
  open: boolean
  onOpenChange: (open: boolean) => void
  /** Existing slugs, to reject a duplicate before the round-trip. */
  existingSlugs: string[]
  /** The created board is selected so the editor opens on it. */
  onCreated: (board: BoardView) => void
}

/** Slugify a display name: lowercase, non-alphanumerics → single dashes. */
function slugify(input: string): string {
  return input
    .toLowerCase()
    .trim()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '')
}

/**
 * Create a new, empty board. It is saved as version 1 through the same
 * `POST /boards` path edits use, then selected so the operator lands in the
 * editor with an empty canvas ready to drag nodes onto. A board needs a trigger
 * on the wire; a new board defaults to `manual` (run it from Test Run) — the
 * cadence is then tuned via Edit.
 */
export function NewBoardDialog({
  open,
  onOpenChange,
  existingSlugs,
  onCreated,
}: NewBoardDialogProps) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className='sm:max-w-md'>
        {open ? (
          <NewBoardBody
            existingSlugs={existingSlugs}
            onOpenChange={onOpenChange}
            onCreated={onCreated}
          />
        ) : null}
      </DialogContent>
    </Dialog>
  )
}

function NewBoardBody({
  existingSlugs,
  onOpenChange,
  onCreated,
}: Omit<NewBoardDialogProps, 'open'>) {
  const save = useSaveBoard()
  const { org, site } = useScope()
  const [name, setName] = useState('')
  // Default to a continuously-running board (interval): a flow that runs 24/7
  // and streams live values, the behaviour an operator expects. "On demand"
  // (manual) is the opt-out for boards you only ever run by hand.
  const [trigger, setTrigger] = useState<Trigger['kind']>('interval')
  const [intervalSecs, setIntervalSecs] = useState(60)
  const [error, setError] = useState<string | null>(null)

  const slug = slugify(name)

  const submit = () => {
    if (!name.trim()) {
      setError('Display name is required.')
      return
    }
    if (!slug) {
      setError('Name must contain at least one letter or number.')
      return
    }
    if (existingSlugs.includes(slug)) {
      setError(`A board "${slug}" already exists.`)
      return
    }
    const triggerValue: Trigger =
      trigger === 'interval'
        ? { kind: 'interval', seconds: Math.max(1, intervalSecs) }
        : { kind: 'manual' }

    if (!org) {
      setError('No org in scope.')
      return
    }
    save.mutate(
      {
        org,
        // A flow created on a site page is scoped to that site.
        site_id: site?.id ?? null,
        slug,
        display_name: name.trim(),
        enabled: true,
        trigger: triggerValue,
        board: { nodes: [], connections: [] },
      },
      {
        onSuccess: (board) => {
          toast('Flow created', { description: `${board.slug} · v${board.version}` })
          onOpenChange(false)
          onCreated(board)
        },
        onError: (e) => setError((e as Error).message),
      }
    )
  }

  return (
    <>
      <DialogHeader>
        <DialogTitle>New flow</DialogTitle>
        <DialogDescription>
          Creates an empty board you can drag nodes onto. A continuous board runs
          on its own and shows live values — no Test Run needed.
        </DialogDescription>
      </DialogHeader>

      <div className='space-y-3 py-1'>
        <div className='space-y-1.5'>
          <Label className='text-[12px]'>Display name</Label>
          <Input
            autoFocus
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder='Night setback'
          />
          {slug ? (
            <p className='text-muted-foreground text-[11px]'>
              slug: <code>{slug}</code>
            </p>
          ) : null}
        </div>

        <div className='space-y-1.5'>
          <Label className='text-[12px]'>Run mode</Label>
          <Select value={trigger} onValueChange={(v) => setTrigger(v as Trigger['kind'])}>
            <SelectTrigger size='sm'>
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value='interval'>Continuous — runs on a fixed interval</SelectItem>
              <SelectItem value='manual'>On demand — only when you run it</SelectItem>
            </SelectContent>
          </Select>
          <p className='text-muted-foreground text-[10.5px]'>
            {trigger === 'interval'
              ? 'The whole board re-evaluates every N seconds, 24/7, and streams live values.'
              : 'The board never runs on its own; use Test Run to evaluate it once.'}
          </p>
        </div>

        {trigger === 'interval' ? (
          <div className='space-y-1.5'>
            <Label className='text-[12px]'>Run every (seconds)</Label>
            <Input
              type='number'
              min={1}
              value={intervalSecs}
              onChange={(e) => setIntervalSecs(Number.parseInt(e.target.value, 10) || 1)}
            />
          </div>
        ) : null}

        {error ? <p className='text-sev-fault text-[12px]'>{error}</p> : null}
      </div>

      <DialogFooter>
        <Button variant='ghost' onClick={() => onOpenChange(false)}>
          Cancel
        </Button>
        <Button onClick={submit} disabled={save.isPending}>
          {save.isPending ? 'Creating…' : 'Create flow'}
        </Button>
      </DialogFooter>
    </>
  )
}
