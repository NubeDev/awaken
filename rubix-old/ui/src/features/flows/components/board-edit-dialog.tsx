import { useState } from 'react'
import { toast } from 'sonner'
import { usePatchBoard, useSaveBoard } from '@/api/hooks'
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
import { Switch } from '@/components/ui/switch'

type BoardEditDialogProps = {
  board: BoardView
  open: boolean
  onOpenChange: (open: boolean) => void
}

/**
 * Edit a board's metadata and run mode. `display_name`/`enabled` are a cheap
 * `PATCH`. Changing the run mode (manual ⇄ continuous, or the interval) rewrites
 * the trigger, which is versioned — so it re-saves the existing graph as a new
 * version through the same path Save uses. Either way the scheduler reconciles
 * the board's loop immediately, so a board can be made live (or paused) here.
 */
export function BoardEditDialog({ board, open, onOpenChange }: BoardEditDialogProps) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className='sm:max-w-md'>
        {/* Mounted per-open so its form state seeds from the current board. */}
        {open ? <BoardEditBody board={board} onOpenChange={onOpenChange} /> : null}
      </DialogContent>
    </Dialog>
  )
}

/** The board's run-mode kind, simplified to the two the editor offers. */
type RunMode = 'interval' | 'manual'

function BoardEditBody({
  board,
  onOpenChange,
}: {
  board: BoardView
  onOpenChange: (open: boolean) => void
}) {
  const patch = usePatchBoard()
  const save = useSaveBoard()

  const [name, setName] = useState(board.display_name)
  const [isEnabled, setIsEnabled] = useState(board.enabled)
  const [mode, setMode] = useState<RunMode>(
    board.trigger.kind === 'interval' ? 'interval' : 'manual'
  )
  const [intervalSecs, setIntervalSecs] = useState(
    board.trigger.kind === 'interval' ? board.trigger.seconds : 60
  )
  const [error, setError] = useState<string | null>(null)

  // A subscription board is shown as manual here (the editor doesn't author
  // subscription keys); we only rewrite the trigger when the operator actually
  // picks a different mode, so such a board is left untouched unless changed.
  const triggerChanged =
    mode !== (board.trigger.kind === 'interval' ? 'interval' : 'manual') ||
    (mode === 'interval' &&
      board.trigger.kind === 'interval' &&
      board.trigger.seconds !== intervalSecs)

  const pending = patch.isPending || save.isPending

  const submit = () => {
    if (!name.trim()) {
      setError('Display name is required.')
      return
    }
    const close = () => onOpenChange(false)

    if (triggerChanged) {
      // Rewriting the trigger is a new version: re-POST the existing graph with
      // the new trigger, carrying the latest name/enabled.
      const trigger: Trigger =
        mode === 'interval'
          ? { kind: 'interval', seconds: Math.max(1, intervalSecs) }
          : { kind: 'manual' }
      save.mutate(
        {
          org: board.org,
          site_id: board.site_id ?? null,
          slug: board.slug,
          display_name: name.trim(),
          enabled: isEnabled,
          trigger,
          board: board.graph,
        },
        {
          onSuccess: (saved) => {
            toast('Run mode updated', { description: `${saved.slug} · v${saved.version}` })
            close()
          },
          onError: (e) => setError((e as Error).message),
        }
      )
      return
    }

    // Metadata-only edit: cheap PATCH, no new version.
    patch.mutate(
      {
        slug: board.slug,
        org: board.org,
        siteId: board.site_id ?? undefined,
        body: { display_name: name.trim(), enabled: isEnabled },
      },
      {
        onSuccess: close,
        onError: (e) => setError((e as Error).message),
      }
    )
  }

  return (
    <>
      <DialogHeader>
        <DialogTitle>Edit board</DialogTitle>
        <DialogDescription>
          <code>{board.slug}</code>. Changing the run mode saves a new version;
          name and enabled are a plain edit.
        </DialogDescription>
      </DialogHeader>

      <div className='space-y-3 py-1'>
        <div className='space-y-1.5'>
          <Label className='text-[12px]'>Display name</Label>
          <Input value={name} onChange={(e) => setName(e.target.value)} />
        </div>

        <div className='space-y-1.5'>
          <Label className='text-[12px]'>Run mode</Label>
          <Select value={mode} onValueChange={(v) => setMode(v as RunMode)}>
            <SelectTrigger size='sm'>
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value='interval'>Continuous — runs on a fixed interval</SelectItem>
              <SelectItem value='manual'>On demand — only when you run it</SelectItem>
            </SelectContent>
          </Select>
          <p className='text-muted-foreground text-[10.5px]'>
            {mode === 'interval'
              ? 'Runs 24/7 and streams live values on the canvas.'
              : 'Never runs on its own; use Test Run to evaluate it once.'}
          </p>
        </div>

        {mode === 'interval' ? (
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

        <div className='flex items-center justify-between'>
          <Label className='text-[12px]'>Enabled (scheduler runs this board)</Label>
          <Switch checked={isEnabled} onCheckedChange={setIsEnabled} />
        </div>

        {error ? <p className='text-sev-fault text-[12px]'>{error}</p> : null}
      </div>

      <DialogFooter>
        <Button variant='ghost' onClick={() => onOpenChange(false)}>
          Cancel
        </Button>
        <Button onClick={submit} disabled={pending}>
          {pending ? 'Saving…' : 'Save changes'}
        </Button>
      </DialogFooter>
    </>
  )
}
