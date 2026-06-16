/**
 * The /wizards landing (WS-06 task 1 wiring): a picker over the onboarding
 * wizards, then the chosen wizard's shell. Each wizard ORCHESTRATES the existing
 * WS-04/05 create paths and applies the standard tags (enums/tags.ts) so
 * dashboards auto-build. See nhp/docs/WIZARDS.md.
 */
import { useState } from 'react'
import {
  Building2,
  MapPin,
  Router,
  Gauge,
  ScanLine,
  Users,
  Workflow,
} from 'lucide-react'
import { Main } from '@/components/layout/main'
import { Button } from '@/components/ui/button'
import {
  Card,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card'
import { GatewayWizard } from './gateway-wizard/gateway-wizard'
import { MetersWizard } from './meters-wizard/meters-wizard'
import { TenantWizard } from './tenant-wizard/tenant-wizard'
import { SiteWizard } from './site-wizard/site-wizard'
import { UserWizard } from './user-wizard/user-wizard'
import { CombinedWizard } from './combined-wizard/combined-wizard'
import { ScanWizard } from './scan-wizard/scan-wizard'

type WizardId =
  | 'tenant'
  | 'site'
  | 'gateway'
  | 'meters'
  | 'scan'
  | 'user'
  | 'combined'
  | null

const WIZARDS = [
  {
    id: 'combined' as const,
    title: 'Add everything (combined)',
    description:
      'Greenfield customer: tenant → site → gateway(+networks) → meters → users in one flow.',
    icon: Workflow,
  },
  {
    id: 'tenant' as const,
    title: 'New tenant',
    description: 'Onboard a customer record (POC: a tagged record, not a namespace).',
    icon: Building2,
  },
  {
    id: 'site' as const,
    title: 'New site',
    description: 'A location under a tenant: name, address, timezone.',
    icon: MapPin,
  },
  {
    id: 'gateway' as const,
    title: 'New gateway + N networks',
    description: 'The "30 networks" flow — generate many networks on a gateway at once.',
    icon: Router,
  },
  {
    id: 'meters' as const,
    title: 'Bulk meters',
    description: 'Add a range of meters onto a network, stamped from a meter-type. Cap-aware.',
    icon: Gauge,
  },
  {
    id: 'scan' as const,
    title: 'Scan to add a device',
    description: 'Scan a meter-type barcode (camera or manual) to add one stamped meter. Cap-aware.',
    icon: ScanLine,
  },
  {
    id: 'user' as const,
    title: 'New user',
    description: 'Add a principal with a role on the rubix admin surface.',
    icon: Users,
  },
]

export function WizardsPage() {
  const [active, setActive] = useState<WizardId>(null)

  if (active) {
    return (
      <Main>
        <Button variant='ghost' className='mb-4' onClick={() => setActive(null)}>
          ← All wizards
        </Button>
        {active === 'tenant' ? <TenantWizard /> : null}
        {active === 'site' ? <SiteWizard /> : null}
        {active === 'gateway' ? <GatewayWizard /> : null}
        {active === 'meters' ? <MetersWizard /> : null}
        {active === 'scan' ? <ScanWizard /> : null}
        {active === 'user' ? <UserWizard /> : null}
        {active === 'combined' ? <CombinedWizard /> : null}
      </Main>
    )
  }

  return (
    <Main>
      <div className='mb-6'>
        <h1 className='text-2xl font-semibold'>Onboarding wizards</h1>
        <p className='text-muted-foreground'>
          Author many gate-audited records and their tags at once. Each wizard
          previews what it will create before writing.
        </p>
      </div>
      <div className='grid gap-4 sm:grid-cols-2 lg:grid-cols-3'>
        {WIZARDS.map((w) => (
          <Card
            key={w.id}
            className='hover:border-primary cursor-pointer transition'
            onClick={() => setActive(w.id)}
          >
            <CardHeader>
              <w.icon className='text-muted-foreground mb-2 h-6 w-6' />
              <CardTitle className='text-base'>{w.title}</CardTitle>
              <CardDescription>{w.description}</CardDescription>
            </CardHeader>
          </Card>
        ))}
      </div>
    </Main>
  )
}
