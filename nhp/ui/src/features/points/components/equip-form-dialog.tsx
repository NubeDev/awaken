import { useState } from 'react'
import { useCreateEquip, usePatchEquip } from '@/api/hooks'
import { parseTags, tagsToInput } from '@/api/tags'
import type { Equip, Uuid } from '@/api/types'
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

type EquipFormDialogProps = {
  open: boolean
  onOpenChange: (open: boolean) => void
} & (
  | { mode: 'create'; siteId: Uuid; equip?: undefined }
  | { mode: 'edit'; equip: Equip; siteId?: undefined }
)

/**
 * Create or edit an equip. `path` is the keyexpr identity (immutable after
 * create), so it is required on create and read-only on edit. `display_name`
 * and `tags` are the editable metadata.
 */
export function EquipFormDialog(props: EquipFormDialogProps) {
  const { open, onOpenChange } = props
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className='sm:max-w-md'>
        {open ? <EquipFormBody {...props} /> : null}
      </DialogContent>
    </Dialog>
  )
}

function EquipFormBody(props: EquipFormDialogProps) {
  const { onOpenChange, mode } = props
  const create = useCreateEquip()
  const patch = usePatchEquip()

  const [path, setPath] = useState(mode === 'edit' ? props.equip.path : '')
  const [displayName, setDisplayName] = useState(
    mode === 'edit' ? props.equip.display_name : ''
  )
  const [tags, setTags] = useState(
    mode === 'edit' ? tagsToInput(props.equip.tags) : ''
  )
  const [error, setError] = useState<string | null>(null)

  const pending = create.isPending || patch.isPending

  const submit = () => {
    if (!displayName.trim()) {
      setError('Display name is required.')
      return
    }
    const onError = (e: unknown) => setError((e as Error).message)
    const onSuccess = () => onOpenChange(false)
    if (mode === 'create') {
      if (!path.trim()) {
        setError('Path is required.')
        return
      }
      create.mutate(
        {
          site_id: props.siteId,
          path: path.trim(),
          display_name: displayName.trim(),
          tags: parseTags(tags),
        },
        { onSuccess, onError }
      )
    } else {
      patch.mutate(
        {
          id: props.equip.id,
          body: { display_name: displayName.trim(), tags: parseTags(tags) },
        },
        { onSuccess, onError }
      )
    }
  }

  return (
    <>
      <DialogHeader>
        <DialogTitle>
          {mode === 'create' ? 'Add equipment' : 'Edit equipment'}
        </DialogTitle>
        <DialogDescription>
          {mode === 'create'
            ? 'A new equip under this site. Path is the keyexpr segment (e.g. ahu-3) and is immutable.'
            : 'Display name and tags are editable; path is immutable identity.'}
        </DialogDescription>
      </DialogHeader>

      <div className='space-y-3 py-1'>
        <div className='space-y-1.5'>
          <Label className='text-[12px]'>Path</Label>
          <Input
            value={path}
            onChange={(e) => setPath(e.target.value)}
            disabled={mode === 'edit'}
            placeholder='ahu-3'
          />
        </div>
        <div className='space-y-1.5'>
          <Label className='text-[12px]'>Display name</Label>
          <Input
            value={displayName}
            onChange={(e) => setDisplayName(e.target.value)}
            placeholder='AHU 3'
          />
        </div>
        <div className='space-y-1.5'>
          <Label className='text-[12px]'>Tags (comma-separated markers)</Label>
          <Input
            value={tags}
            onChange={(e) => setTags(e.target.value)}
            placeholder='ahu, rooftop'
          />
        </div>
        {error ? <p className='text-[12px] text-sev-fault'>{error}</p> : null}
      </div>

      <DialogFooter>
        <Button variant='ghost' onClick={() => onOpenChange(false)}>
          Cancel
        </Button>
        <Button onClick={submit} disabled={pending}>
          {pending
            ? 'Saving…'
            : mode === 'create'
              ? 'Create equip'
              : 'Save changes'}
        </Button>
      </DialogFooter>
    </>
  )
}
