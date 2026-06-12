import { Main } from '@/components/layout/main'
import { PageHeader } from '@/components/layout/page-header'
import { useActiveSite } from '@/hooks/use-active-site'
import { DemandChart } from './components/demand-chart'
import { EquipmentHealth } from './components/equipment-health'
import { KpiRow } from './components/kpi-row'
import { LoadBreakdown } from './components/load-breakdown'
import { RecentSparks } from './components/recent-sparks'

export function Dashboard() {
  const { site } = useActiveSite()

  return (
    <>
      <PageHeader
        title='Dashboard'
        sub={site ? `${site.display_name} · live overview` : 'Loading site…'}
      />
      <Main fluid>
        <div className='space-y-4'>
          <KpiRow siteId={site?.id} />
          <div className='grid gap-4 lg:grid-cols-3'>
            <DemandChart siteId={site?.id} />
            <LoadBreakdown siteId={site?.id} />
          </div>
          <div className='grid gap-4 lg:grid-cols-3'>
            <EquipmentHealth siteId={site?.id} />
            <RecentSparks siteId={site?.id} />
          </div>
        </div>
      </Main>
    </>
  )
}
