/**
 * Create / edit a `kind:"site"` (DOMAIN-MODEL §site): key, name, tenant (the
 * REQUIRED parent relation), address, IANA timezone, and `geo` ("lat,lng").
 * Location entry is assisted: typing the address autocompletes via OSM Nominatim,
 * and picking a match fills the coordinates AND derives the timezone (offline
 * tz-lookup) — both still editable, with a draggable map pin to fine-tune. The
 * tenant picker lists tenants by record id (relations are by id). On edit the
 * existing content is spread back so any tags survive the PATCH untouched.
 */
import { useState } from 'react'
import tzLookup from 'tz-lookup'
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
import { AddressAutocomplete } from './address-autocomplete'
import { formatGeo, parseGeo, type LatLng } from './geo'
import { SiteLocationMap } from './site-location-map'
import { TimezoneSelect } from './timezone-select'
import { useCreateSite, useTenants, useUpdateSite } from './hooks'

type SiteFormProps = {
  open: boolean
  onOpenChange: (open: boolean) => void
  /** Present = edit; absent = create. */
  site?: SiteRecord
}

/** IANA timezone at a coordinate, or '' if the lookup can't place it (open ocean). */
function tzAt({ lat, lng }: LatLng): string {
  try {
    return tzLookup(lat, lng)
  } catch {
    return ''
  }
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

  const coord = parseGeo(geo)

  // Set coordinates from anywhere (address pick / map drag / map click) and
  // auto-fill the timezone from those coordinates so the admin rarely types it.
  const applyCoord = (next: LatLng) => {
    setGeo(formatGeo(next))
    const tz = tzAt(next)
    if (tz) setTimezone(tz)
  }

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
      <DialogContent className='max-h-[90vh] max-w-lg overflow-y-auto'>
        <DialogHeader>
          <DialogTitle>{editing ? 'Edit site' : 'New site'}</DialogTitle>
          <DialogDescription>
            A physical location under a tenant. Hosts gateways; its timezone
            drives site-local time on dashboards.
          </DialogDescription>
        </DialogHeader>

        <div className='grid gap-4'>
          <div className='grid grid-cols-2 gap-4'>
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
            <AddressAutocomplete
              id='site-address'
              value={address}
              onChange={setAddress}
              onPick={(r) => applyCoord({ lat: r.lat, lng: r.lng })}
              placeholder='Start typing… e.g. 1 Market St, Springfield'
            />
            <p className='text-muted-foreground text-xs'>
              Searches OpenStreetMap; picking a match fills the map & timezone.
            </p>
          </div>

          <SiteLocationMap value={coord} onChange={applyCoord} />

          <div className='grid grid-cols-2 gap-4'>
            <div className='grid gap-1'>
              <Label htmlFor='site-geo'>Coordinates</Label>
              <Input
                id='site-geo'
                value={geo}
                onChange={(e) => setGeo(e.target.value)}
                placeholder='lat,lng'
              />
              <p className='text-muted-foreground text-xs'>
                latitude,longitude
              </p>
            </div>
            <div className='grid gap-1'>
              <Label htmlFor='site-timezone'>Timezone</Label>
              <TimezoneSelect
                id='site-timezone'
                value={timezone}
                onChange={setTimezone}
              />
              <p className='text-muted-foreground text-xs'>
                Auto-set from the map; editable.
              </p>
            </div>
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
