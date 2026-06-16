/**
 * Create / edit a `kind:"site"` (DOMAIN-MODEL §site): key, name, tenant (the
 * REQUIRED parent relation), address, and IANA timezone (dashboards render
 * site-local time). The tenant picker lists `kind:"tenant"` records by their
 * record id (relations are by id, per the WS-03 seed). On edit the existing
 * content is spread back so any tags survive the PATCH untouched.
 */
import { useState } from 'react'
import type { Site, SiteRecord } from '@/api/records'
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
import { useCreateSite, useTenants, useUpdateSite } from './hooks'

type SiteFormProps = {
  open: boolean
  onOpenChange: (open: boolean) => void
  /** Present = edit; absent = create. */
  site?: SiteRecord
}

export function SiteForm({ open, onOpenChange, site }: SiteFormProps) {
  const editing = site !== undefined
  const start = site?.content
  const [key, setKey] = useState(start?.key ?? '')
  const [name, setName] = useState(start?.name ?? '')
  const [tenant, setTenant] = useState(start?.tenant ?? '')
  const [address, setAddress] = useState(start?.address ?? '')
  const [timezone, setTimezone] = useState(start?.timezone ?? '')
  const [geo, setGeo] = useState(start?.geo ?? '')

  const create = useCreateSite()
  const update = useUpdateSite()
  const tenants = useTenants()
  const pending = create.isPending || update.isPending
  // tenant is the required parent relation.
  const valid = key.trim() !== '' && name.trim() !== '' && tenant !== ''

  const save = () => {
    if (editing && site) {
      const content: Site = {
        ...site.content,
        key,
        name,
        tenant,
        address,
        timezone,
        geo,
      }
      update.mutate(
        { id: site.id, content },
        { onSuccess: () => onOpenChange(false) }
      )
      return
    }
    const content: Site = {
      kind: 'site',
      key,
      name,
      tenant,
      address,
      timezone,
      geo,
    }
    create.mutate(content, { onSuccess: () => onOpenChange(false) })
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className='max-w-md'>
        <DialogHeader>
          <DialogTitle>{editing ? 'Edit site' : 'New site'}</DialogTitle>
          <DialogDescription>
            A physical location under a tenant. Hosts gateways; its timezone
            drives site-local time on dashboards.
          </DialogDescription>
        </DialogHeader>

        <div className='grid gap-4'>
          <div className='grid gap-1'>
            <Label htmlFor='site-key'>Key</Label>
            <Input
              id='site-key'
              value={key}
              onChange={(e) => setKey(e.target.value)}
              placeholder='acme-hq'
            />
          </div>
          <div className='grid gap-1'>
            <Label htmlFor='site-name'>Name</Label>
            <Input
              id='site-name'
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder='Acme HQ'
            />
          </div>
          <div className='grid gap-1'>
            <Label htmlFor='site-tenant'>Tenant</Label>
            <Select value={tenant} onValueChange={setTenant}>
              <SelectTrigger id='site-tenant'>
                <SelectValue placeholder='Select a tenant' />
              </SelectTrigger>
              <SelectContent>
                {(tenants.data ?? []).map((t) => (
                  <SelectItem key={t.id} value={t.id}>
                    {t.content.name} ({t.content.key})
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
          <div className='grid gap-1'>
            <Label htmlFor='site-address'>Address</Label>
            <Input
              id='site-address'
              value={address}
              onChange={(e) => setAddress(e.target.value)}
              placeholder='1 Market St, Springfield'
            />
          </div>
          <div className='grid gap-1'>
            <Label htmlFor='site-timezone'>Timezone</Label>
            <Input
              id='site-timezone'
              value={timezone}
              onChange={(e) => setTimezone(e.target.value)}
              placeholder='America/New_York'
            />
          </div>
          <div className='grid gap-1'>
            <Label htmlFor='site-geo'>Coordinates</Label>
            <Input
              id='site-geo'
              value={geo}
              onChange={(e) => setGeo(e.target.value)}
              placeholder='lat,lng — e.g. 39.7817,-89.6501'
            />
            <p className='text-muted-foreground text-xs'>
              Plots the site on the map. Format: latitude,longitude.
            </p>
          </div>
        </div>

        <DialogFooter>
          <Button variant='ghost' onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button onClick={save} disabled={!valid || pending}>
            {pending ? 'Saving…' : 'Save'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
