import { useState } from 'react'
import { useProvisionOrg } from '@/api/hooks'
import { parseTags } from '@/api/tags'
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

type ProvisionTenantDialogProps = {
  open: boolean
  onOpenChange: (open: boolean) => void
}

/**
 * Onboard a tenant in one action: `POST /orgs` creates the org's first site.
 * There is no separate org entity — the org is the namespace string carried by
 * its sites — so a tenant exists once its first site does.
 */
export function ProvisionTenantDialog({
  open,
  onOpenChange,
}: ProvisionTenantDialogProps) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className='sm:max-w-md'>
        {/* Mounted per-open so the form starts fresh — no resetting effect. */}
        {open ? <ProvisionTenantBody onOpenChange={onOpenChange} /> : null}
      </DialogContent>
    </Dialog>
  )
}

function ProvisionTenantBody({
  onOpenChange,
}: {
  onOpenChange: (open: boolean) => void
}) {
  const provision = useProvisionOrg()
  const [org, setOrg] = useState('')
  const [slug, setSlug] = useState('')
  const [displayName, setDisplayName] = useState('')
  const [tags, setTags] = useState('site')
  const [error, setError] = useState<string | null>(null)

  const submit = () => {
    if (!org.trim() || !slug.trim() || !displayName.trim()) {
      setError('Org, slug, and display name are all required.')
      return
    }
    provision.mutate(
      {
        org: org.trim(),
        slug: slug.trim(),
        display_name: displayName.trim(),
        tags: parseTags(tags),
      },
      {
        onSuccess: () => onOpenChange(false),
        onError: (e) => setError((e as Error).message),
      }
    )
  }

  return (
    <>
      <DialogHeader>
        <DialogTitle>New tenant</DialogTitle>
        <DialogDescription>
          Provision an org and its first site in one step. The org is a
          lowercase namespace (slug rules); both org and slug are immutable.
        </DialogDescription>
      </DialogHeader>

      <div className='space-y-3 py-1'>
        <div className='grid grid-cols-2 gap-2'>
          <Field label='Org'>
            <Input
              value={org}
              onChange={(e) => setOrg(e.target.value)}
              placeholder='kfc'
            />
          </Field>
          <Field label='First site slug'>
            <Input
              value={slug}
              onChange={(e) => setSlug(e.target.value)}
              placeholder='hq'
            />
          </Field>
        </div>
        <Field label='Site display name'>
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
            placeholder='site'
          />
        </Field>
        {error ? <p className='text-[12px] text-sev-fault'>{error}</p> : null}
      </div>

      <DialogFooter>
        <Button variant='ghost' onClick={() => onOpenChange(false)}>
          Cancel
        </Button>
        <Button onClick={submit} disabled={provision.isPending}>
          {provision.isPending ? 'Provisioning…' : 'Provision tenant'}
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
