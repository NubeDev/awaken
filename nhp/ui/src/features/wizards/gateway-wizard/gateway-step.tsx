/**
 * Step 1 of the gateway wizard: the gateway's own fields + its parent site. The
 * site is a REQUIRED relation (WS-05: the gate rejects a gateway without one), so
 * it is a picker over existing `kind:"site"` records; selecting it also captures
 * the site KEY + tenant KEY needed for the standard tags. Controlled — owns no
 * state; the wizard holds it.
 */
import type { SiteRecord } from '@/api/records'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import type { TenantContent } from '../_shared/hooks'
import type { RecordDto } from '@/api/records'

export interface GatewayStepValue {
  key: string
  name: string
  model: string
  host: string
  /** Selected site RECORD id. */
  siteId: string
}

export function GatewayStep({
  value,
  onChange,
  sites,
  tenants,
}: {
  value: GatewayStepValue
  onChange: (next: GatewayStepValue) => void
  sites: SiteRecord[]
  tenants: RecordDto<TenantContent>[]
}) {
  // Map a site record → its tenant key, for the tag context (resolved at plan time).
  return (
    <div className='grid gap-4'>
      <div className='grid gap-4 sm:grid-cols-2'>
        <div className='grid gap-1'>
          <Label htmlFor='gw-key'>Key</Label>
          <Input
            id='gw-key'
            value={value.key}
            onChange={(e) => onChange({ ...value, key: e.target.value })}
            placeholder='gw-01'
          />
        </div>
        <div className='grid gap-1'>
          <Label htmlFor='gw-name'>Name</Label>
          <Input
            id='gw-name'
            value={value.name}
            onChange={(e) => onChange({ ...value, name: e.target.value })}
            placeholder='Basement gateway'
          />
        </div>
      </div>

      <div className='grid gap-1'>
        <Label htmlFor='gw-site'>Site</Label>
        <Select
          value={value.siteId}
          onValueChange={(siteId) => onChange({ ...value, siteId })}
        >
          <SelectTrigger id='gw-site'>
            <SelectValue placeholder='Select the parent site' />
          </SelectTrigger>
          <SelectContent>
            {sites.map((s) => (
              <SelectItem key={s.id} value={s.id}>
                {s.content.name} ({s.content.key})
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        {tenants.length === 0 ? (
          <p className='text-muted-foreground text-xs'>
            No tenants yet — use the tenant or combined wizard first.
          </p>
        ) : null}
      </div>

      <div className='grid gap-4 sm:grid-cols-2'>
        <div className='grid gap-1'>
          <Label htmlFor='gw-model'>Model</Label>
          <Input
            id='gw-model'
            value={value.model}
            onChange={(e) => onChange({ ...value, model: e.target.value })}
            placeholder='NHP-GW-4'
          />
        </div>
        <div className='grid gap-1'>
          <Label htmlFor='gw-host'>Host</Label>
          <Input
            id='gw-host'
            value={value.host}
            onChange={(e) => onChange({ ...value, host: e.target.value })}
            placeholder='10.0.0.1 (the poller uses this)'
          />
        </div>
      </div>
    </div>
  )
}

export function gatewayStepValid(v: GatewayStepValue): boolean {
  return v.key.trim() !== '' && v.name.trim() !== '' && v.siteId !== ''
}
