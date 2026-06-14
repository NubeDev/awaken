import { useState } from 'react'
import { useCreateDashboard, usePatchDashboard } from '@/api/hooks'
import type { Dashboard, Site } from '@/api/types'
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

const OVERVIEW = '__overview__'

type DashboardFormDialogProps = {
  open: boolean
  onOpenChange: (open: boolean) => void
  /** The org these boards belong to (the active site's org). */
  org: string
  /** Sites the operator can scope a board to. */
  sites: Site[]
  /** When set, a created board's id is reported so the caller can select it. */
  onCreated?: (id: string) => void
} & (
  | { mode: 'create'; dashboard?: undefined }
  | { mode: 'edit'; dashboard: Dashboard }
)

/**
 * Create or rename a dashboard. On create the operator picks the scope — an
 * **org overview** (spans every site) or a single **site** — plus a slug; both
 * are immutable afterwards, so edit only renames the title.
 */
export function DashboardFormDialog(props: DashboardFormDialogProps) {
  const { open, onOpenChange } = props
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className='sm:max-w-md'>
        {open ? <Body {...props} /> : null}
      </DialogContent>
    </Dialog>
  )
}

function Body(props: DashboardFormDialogProps) {
  const { mode, org, sites, onOpenChange, onCreated } = props
  const create = useCreateDashboard()
  const patch = usePatchDashboard()

  const [title, setTitle] = useState(
    mode === 'edit' ? props.dashboard.title : ''
  )
  const [slug, setSlug] = useState('')
  const [scope, setScope] = useState<string>(OVERVIEW)
  const [error, setError] = useState<string | null>(null)

  const pending = create.isPending || patch.isPending

  const submit = () => {
    if (!title.trim()) {
      setError('Title is required.')
      return
    }
    const onError = (e: unknown) => setError((e as Error).message)
    if (mode === 'edit') {
      patch.mutate(
        { id: props.dashboard.id, body: { title: title.trim() } },
        { onSuccess: () => onOpenChange(false), onError }
      )
      return
    }
    if (!slug.trim()) {
      setError('Slug is required.')
      return
    }
    create.mutate(
      {
        org,
        site_id: scope === OVERVIEW ? null : scope,
        slug: slug.trim(),
        title: title.trim(),
      },
      {
        onSuccess: (d) => {
          onCreated?.(d.id)
          onOpenChange(false)
        },
        onError,
      }
    )
  }

  return (
    <>
      <DialogHeader>
        <DialogTitle>
          {mode === 'create' ? 'New dashboard' : 'Rename dashboard'}
        </DialogTitle>
        <DialogDescription>
          {mode === 'create'
            ? 'An org overview spans every site; a site board is scoped to one. Scope and slug are immutable.'
            : 'Only the title is editable; scope and slug are fixed identity.'}
        </DialogDescription>
      </DialogHeader>

      <div className='space-y-3 py-1'>
        <div className='space-y-1.5'>
          <Label className='text-[12px]'>Title</Label>
          <Input
            value={title}
            onChange={(e) => setTitle(e.target.value)}
            placeholder='Portfolio Overview'
          />
        </div>

        {mode === 'create' ? (
          <>
            <div className='space-y-1.5'>
              <Label className='text-[12px]'>Scope</Label>
              <Select value={scope} onValueChange={setScope}>
                <SelectTrigger size='sm' className='w-full'>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value={OVERVIEW}>
                    Org overview ({org}) — spans sites
                  </SelectItem>
                  {sites.map((s) => (
                    <SelectItem key={s.id} value={s.id}>
                      Site · {s.display_name}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
            <div className='space-y-1.5'>
              <Label className='text-[12px]'>Slug</Label>
              <Input
                value={slug}
                onChange={(e) => setSlug(e.target.value)}
                placeholder='portfolio'
              />
            </div>
          </>
        ) : null}

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
              ? 'Create dashboard'
              : 'Save'}
        </Button>
      </DialogFooter>
    </>
  )
}
