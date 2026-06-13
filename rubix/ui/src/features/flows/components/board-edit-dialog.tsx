import { useState } from 'react'
import { usePatchBoard } from '@/api/hooks'
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
import { Switch } from '@/components/ui/switch'

type BoardEditDialogProps = {
  slug: string
  displayName: string
  enabled: boolean
  open: boolean
  onOpenChange: (open: boolean) => void
}

/**
 * Edit a board's latest-version metadata (`display_name`, `enabled`) via
 * `PATCH /boards/{slug}`. The graph and trigger are versioned and edited
 * through Save (a new version), not here. Toggling enabled takes effect on the
 * next scheduler launch.
 */
export function BoardEditDialog({
  open,
  onOpenChange,
  ...seed
}: BoardEditDialogProps) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className='sm:max-w-md'>
        {/* Body is keyed/mounted per-open so its form state seeds from the
            current board without a state-syncing effect. */}
        {open ? <BoardEditBody {...seed} onOpenChange={onOpenChange} /> : null}
      </DialogContent>
    </Dialog>
  )
}

function BoardEditBody({
  slug,
  displayName,
  enabled,
  onOpenChange,
}: Omit<BoardEditDialogProps, 'open'>) {
  const patch = usePatchBoard()
  const [name, setName] = useState(displayName)
  const [isEnabled, setIsEnabled] = useState(enabled)
  const [error, setError] = useState<string | null>(null)

  const submit = () => {
    if (!name.trim()) {
      setError('Display name is required.')
      return
    }
    patch.mutate(
      { slug, body: { display_name: name.trim(), enabled: isEnabled } },
      {
        onSuccess: () => onOpenChange(false),
        onError: (e) => setError((e as Error).message),
      }
    )
  }

  return (
    <>
      <DialogHeader>
        <DialogTitle>Edit board</DialogTitle>
        <DialogDescription>
          Metadata for <code>{slug}</code>. Graph and trigger edits save as a
          new version.
        </DialogDescription>
      </DialogHeader>

      <div className='space-y-3 py-1'>
        <div className='space-y-1.5'>
          <Label className='text-[12px]'>Display name</Label>
          <Input value={name} onChange={(e) => setName(e.target.value)} />
        </div>
        <div className='flex items-center justify-between'>
          <Label className='text-[12px]'>
            Enabled (scheduler fires this board)
          </Label>
          <Switch checked={isEnabled} onCheckedChange={setIsEnabled} />
        </div>
        {error ? <p className='text-[12px] text-sev-fault'>{error}</p> : null}
      </div>

      <DialogFooter>
        <Button variant='ghost' onClick={() => onOpenChange(false)}>
          Cancel
        </Button>
        <Button onClick={submit} disabled={patch.isPending}>
          {patch.isPending ? 'Saving…' : 'Save changes'}
        </Button>
      </DialogFooter>
    </>
  )
}
