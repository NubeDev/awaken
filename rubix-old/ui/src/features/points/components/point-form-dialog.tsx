import { useState } from 'react'
import { useCreatePoint, usePatchPoint } from '@/api/hooks'
import { parseTags, tagsToInput } from '@/api/tags'
import type { Point, PointKind, Uuid } from '@/api/types'
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

const KINDS: { value: PointKind; label: string }[] = [
  { value: 'sensor', label: 'Sensor (read-only)' },
  { value: 'sp', label: 'Setpoint (writable)' },
  { value: 'cmd', label: 'Command (writable)' },
]

type PointFormDialogProps = {
  open: boolean
  onOpenChange: (open: boolean) => void
} & (
  | { mode: 'create'; equipId: Uuid; point?: undefined }
  | { mode: 'edit'; point: Point; equipId?: undefined }
)

/**
 * Create or edit a point. `slug` is the keyexpr identity (immutable after
 * create). On create, kind is fixed (it decides writability); on edit kind,
 * unit, display_name and tags are all mutable metadata.
 */
export function PointFormDialog(props: PointFormDialogProps) {
  const { open, onOpenChange } = props
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className='sm:max-w-md'>
        {open ? <PointFormBody {...props} /> : null}
      </DialogContent>
    </Dialog>
  )
}

function PointFormBody(props: PointFormDialogProps) {
  const { onOpenChange, mode } = props
  const create = useCreatePoint()
  const patch = usePatchPoint()

  const [slug, setSlug] = useState(mode === 'edit' ? props.point.slug : '')
  const [displayName, setDisplayName] = useState(
    mode === 'edit' ? props.point.display_name : ''
  )
  const [kind, setKind] = useState<PointKind>(
    mode === 'edit' ? props.point.kind : 'sensor'
  )
  const [unit, setUnit] = useState(
    mode === 'edit' ? (props.point.unit ?? '') : ''
  )
  const [tags, setTags] = useState(
    mode === 'edit' ? tagsToInput(props.point.tags) : ''
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
    const trimmedUnit = unit.trim()
    if (mode === 'create') {
      if (!slug.trim()) {
        setError('Slug is required.')
        return
      }
      create.mutate(
        {
          equip_id: props.equipId,
          slug: slug.trim(),
          display_name: displayName.trim(),
          kind,
          unit: trimmedUnit || null,
          tags: parseTags(tags),
        },
        { onSuccess, onError }
      )
    } else {
      patch.mutate(
        {
          id: props.point.id,
          body: {
            display_name: displayName.trim(),
            kind,
            // unit is set-only via PATCH; omit when blank to leave unchanged.
            ...(trimmedUnit ? { unit: trimmedUnit } : {}),
            tags: parseTags(tags),
          },
        },
        { onSuccess, onError }
      )
    }
  }

  return (
    <>
      <DialogHeader>
        <DialogTitle>
          {mode === 'create' ? 'Add point' : 'Edit point'}
        </DialogTitle>
        <DialogDescription>
          {mode === 'create'
            ? 'A new point under this equip. Slug is the keyexpr segment and is immutable.'
            : 'Metadata is editable; slug is immutable identity.'}
        </DialogDescription>
      </DialogHeader>

      <div className='space-y-3 py-1'>
        <div className='grid grid-cols-2 gap-2'>
          <div className='space-y-1.5'>
            <Label className='text-[12px]'>Slug</Label>
            <Input
              value={slug}
              onChange={(e) => setSlug(e.target.value)}
              disabled={mode === 'edit'}
              placeholder='discharge-temp'
            />
          </div>
          <div className='space-y-1.5'>
            <Label className='text-[12px]'>Kind</Label>
            <Select value={kind} onValueChange={(v) => setKind(v as PointKind)}>
              <SelectTrigger size='sm' className='w-full'>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {KINDS.map((k) => (
                  <SelectItem key={k.value} value={k.value}>
                    {k.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
        </div>
        <div className='grid grid-cols-2 gap-2'>
          <div className='space-y-1.5'>
            <Label className='text-[12px]'>Display name</Label>
            <Input
              value={displayName}
              onChange={(e) => setDisplayName(e.target.value)}
              placeholder='Discharge Temp'
            />
          </div>
          <div className='space-y-1.5'>
            <Label className='text-[12px]'>Unit</Label>
            <Input
              value={unit}
              onChange={(e) => setUnit(e.target.value)}
              placeholder='°C'
            />
          </div>
        </div>
        <div className='space-y-1.5'>
          <Label className='text-[12px]'>Tags (comma-separated markers)</Label>
          <Input
            value={tags}
            onChange={(e) => setTags(e.target.value)}
            placeholder='sp, trim'
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
              ? 'Create point'
              : 'Save changes'}
        </Button>
      </DialogFooter>
    </>
  )
}
