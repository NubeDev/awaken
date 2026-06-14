import { useState } from 'react'
import { useCreateSite, usePatchSite } from '@/api/hooks'
import { parseTags, tagsToInput } from '@/api/tags'
import type { Site } from '@/api/types'
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

type SiteFormDialogProps = {
  open: boolean
  onOpenChange: (open: boolean) => void
} & (
  | { mode: 'create'; org: string; site?: undefined }
  | { mode: 'edit'; site: Site; org?: undefined }
)

/**
 * Create or edit a site. On create, `org` is fixed (the tenant it's added to)
 * and `slug` is required — both are identity and immutable afterwards. On edit
 * only `display_name` and `tags` are mutable (the backend rejects org/slug
 * changes with 400), so those fields are read-only.
 */
export function SiteFormDialog(props: SiteFormDialogProps) {
  const { open, onOpenChange } = props
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className='sm:max-w-md'>
        {/* Mounted per-open so the form seeds from props initializers — no
            state-syncing effect. */}
        {open ? <SiteFormBody {...props} /> : null}
      </DialogContent>
    </Dialog>
  )
}

function SiteFormBody(props: SiteFormDialogProps) {
  const { onOpenChange, mode } = props
  const create = useCreateSite()
  const patch = usePatchSite()

  const [slug, setSlug] = useState(mode === 'edit' ? props.site.slug : '')
  const [displayName, setDisplayName] = useState(
    mode === 'edit' ? props.site.display_name : ''
  )
  const [tags, setTags] = useState(
    mode === 'edit' ? tagsToInput(props.site.tags) : ''
  )
  const [error, setError] = useState<string | null>(null)

  const pending = create.isPending || patch.isPending
  const org = mode === 'create' ? props.org : props.site.org

  const submit = () => {
    if (!displayName.trim()) {
      setError('Display name is required.')
      return
    }
    const onError = (e: unknown) => setError((e as Error).message)
    const onSuccess = () => onOpenChange(false)
    if (mode === 'create') {
      if (!slug.trim()) {
        setError('Slug is required.')
        return
      }
      create.mutate(
        {
          org,
          slug: slug.trim(),
          display_name: displayName.trim(),
          tags: parseTags(tags),
        },
        { onSuccess, onError }
      )
    } else {
      patch.mutate(
        {
          id: props.site.id,
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
          {mode === 'create' ? 'Add site' : 'Edit site'}
        </DialogTitle>
        <DialogDescription>
          {mode === 'create'
            ? `A new site under ${org}. org and slug are immutable once created.`
            : 'Display name and tags are editable; org and slug are immutable identity.'}
        </DialogDescription>
      </DialogHeader>

      <div className='space-y-3 py-1'>
        <div className='grid grid-cols-2 gap-2'>
          <Field label='Org'>
            <Input value={org} disabled />
          </Field>
          <Field label='Slug'>
            <Input
              value={slug}
              onChange={(e) => setSlug(e.target.value)}
              disabled={mode === 'edit'}
              placeholder='hq'
            />
          </Field>
        </div>
        <Field label='Display name'>
          <Input
            value={displayName}
            onChange={(e) => setDisplayName(e.target.value)}
            placeholder='KFC HQ'
          />
        </Field>
        <Field label='Tags (comma-separated markers)'>
          <Input
            value={tags}
            onChange={(e) => setTags(e.target.value)}
            placeholder='site, flagship'
          />
        </Field>
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
              ? 'Create site'
              : 'Save changes'}
        </Button>
      </DialogFooter>
    </>
  )
}

function Field({
  label,
  children,
}: {
  label: string
  children: React.ReactNode
}) {
  return (
    <div className='space-y-1.5'>
      <Label className='text-[12px]'>{label}</Label>
      {children}
    </div>
  )
}
