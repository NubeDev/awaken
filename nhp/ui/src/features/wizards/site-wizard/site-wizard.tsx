/**
 * New-site wizard (WS-06 task 4): a location under a tenant — name/address/
 * timezone (timezone drives dashboard site-local time, DOMAIN-MODEL §site). The
 * `tenant` field is a relation to an existing tenant RECORD id; the tenant KEY is
 * captured for the standard `tenant:<key>` tag (enums/tags.ts siteTags), matching
 * the seed (portfolio.mjs).
 */
import { useState } from 'react'
import type { Site } from '@/api/records'
import { siteTags } from '@/enums/tags'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { WizardShell } from '../_shared/stepper'
import type { PlannedRecord } from '../_shared/batch-write'
import { useTenants } from '../_shared/hooks'

export function SiteWizard() {
  const tenants = useTenants()
  const [key, setKey] = useState('')
  const [name, setName] = useState('')
  const [tenantId, setTenantId] = useState('')
  const [address, setAddress] = useState('')
  const [timezone, setTimezone] = useState('UTC')

  const tenantList = tenants.data ?? []
  const tenantKey =
    tenantList.find((t) => t.id === tenantId)?.content.key ?? ''

  const buildPlan = (): PlannedRecord[] => {
    const content: Site & { address?: string; timezone?: string } = {
      kind: 'site',
      key,
      name,
      tenant: tenantId, // relation by record id (matches WS-03 portfolio.mjs)
      address,
      timezone,
      tags: siteTags({ tenant: tenantKey }),
    }
    return [
      {
        id: 'site',
        label: `site ${key}`,
        kind: 'site',
        content: content as unknown as Record<string, unknown>,
      },
    ]
  }

  return (
    <WizardShell
      title='New site'
      description='A physical location under a tenant.'
      steps={[
        {
          title: 'Site',
          valid: key.trim() !== '' && name.trim() !== '' && tenantId !== '',
          render: () => (
            <div className='grid gap-4'>
              <div className='grid gap-1'>
                <Label htmlFor='s-tenant'>Tenant</Label>
                <Select value={tenantId} onValueChange={setTenantId}>
                  <SelectTrigger id='s-tenant'>
                    <SelectValue placeholder='Select the parent tenant' />
                  </SelectTrigger>
                  <SelectContent>
                    {tenantList.map((t) => (
                      <SelectItem key={t.id} value={t.id}>
                        {t.content.name} ({t.content.key})
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
              <div className='grid gap-4 sm:grid-cols-2'>
                <div className='grid gap-1'>
                  <Label htmlFor='s-key'>Key</Label>
                  <Input
                    id='s-key'
                    value={key}
                    onChange={(e) => setKey(e.target.value)}
                    placeholder='hq'
                  />
                </div>
                <div className='grid gap-1'>
                  <Label htmlFor='s-name'>Name</Label>
                  <Input
                    id='s-name'
                    value={name}
                    onChange={(e) => setName(e.target.value)}
                    placeholder='Head office'
                  />
                </div>
              </div>
              <div className='grid gap-4 sm:grid-cols-2'>
                <div className='grid gap-1'>
                  <Label htmlFor='s-address'>Address</Label>
                  <Input
                    id='s-address'
                    value={address}
                    onChange={(e) => setAddress(e.target.value)}
                    placeholder='1 Main St'
                  />
                </div>
                <div className='grid gap-1'>
                  <Label htmlFor='s-tz'>Timezone (IANA)</Label>
                  <Input
                    id='s-tz'
                    value={timezone}
                    onChange={(e) => setTimezone(e.target.value)}
                    placeholder='Australia/Sydney'
                  />
                </div>
              </div>
            </div>
          ),
        },
      ]}
      buildPlan={buildPlan}
    />
  )
}
