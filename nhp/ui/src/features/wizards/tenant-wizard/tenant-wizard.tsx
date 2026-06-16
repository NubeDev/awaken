/**
 * New-tenant wizard (WS-06 task 4). TENANT DECISION (WS-03 §Notes, confirmed):
 * in the POC NHP tenants are RECORDS (kind:"tenant") in the single `acme`
 * namespace, distinguished by `tenant:<key>` tags — NOT separate rubix namespaces.
 * True per-tenant namespace isolation is a multi-tenant-onboarding concern beyond
 * the POC's data-to-render goal. So this wizard writes one `kind:"tenant"` record
 * (key/name), mirroring the seed's tenant records (portfolio.mjs). The first-admin
 * / namespace-provisioning steps from WIZARDS.md §1 are out of POC scope (no
 * fresh-namespace path); see the user wizard for adding principals.
 *
 * A tenant is the hierarchy root, so it carries no ancestor tags (tags: []), same
 * as the seed.
 */
import { useState } from 'react'
import { WizardShell } from '../_shared/stepper'
import type { PlannedRecord } from '../_shared/batch-write'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'

export function TenantWizard() {
  const [key, setKey] = useState('')
  const [name, setName] = useState('')

  const buildPlan = (): PlannedRecord[] => [
    {
      id: 'tenant',
      label: `tenant ${key}`,
      kind: 'tenant',
      content: {
        kind: 'tenant',
        key,
        name,
        // POC single namespace: every NHP record lives in `acme` (WS-03 decision).
        namespace: 'acme',
        tags: [],
      },
    },
  ]

  return (
    <WizardShell
      title='New tenant'
      description='Onboard a customer. In the POC a tenant is a tagged record in the shared namespace.'
      steps={[
        {
          title: 'Tenant',
          valid: key.trim() !== '' && name.trim() !== '',
          render: () => (
            <div className='grid gap-4 sm:grid-cols-2'>
              <div className='grid gap-1'>
                <Label htmlFor='t-key'>Key</Label>
                <Input
                  id='t-key'
                  value={key}
                  onChange={(e) => setKey(e.target.value)}
                  placeholder='acme'
                />
              </div>
              <div className='grid gap-1'>
                <Label htmlFor='t-name'>Name</Label>
                <Input
                  id='t-name'
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                  placeholder='Acme Corp'
                />
              </div>
            </div>
          ),
        },
      ]}
      buildPlan={buildPlan}
    />
  )
}
