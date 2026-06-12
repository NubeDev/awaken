import { ConfigDrawer } from '@/components/config-drawer'
import { Header } from '@/components/layout/header'
import { Main } from '@/components/layout/main'
import { ProfileDropdown } from '@/components/profile-dropdown'
import { Search } from '@/components/search'
import { ThemeSwitch } from '@/components/theme-switch'
import { useActiveSite } from '@/hooks/use-active-site'
import { DemandChart } from './components/demand-chart'
import { EquipmentHealth } from './components/equipment-health'
import { KpiRow } from './components/kpi-row'
import { RecentSparks } from './components/recent-sparks'

export function Dashboard() {
  const { site } = useActiveSite()

  return (
    <>
      <Header>
        <Search />
        <div className='ms-auto flex items-center gap-2'>
          <ThemeSwitch />
          <ConfigDrawer />
          <ProfileDropdown />
        </div>
      </Header>

      <Main>
        <div className='mb-4 flex items-center justify-between'>
          <div>
            <h1 className='text-2xl font-bold tracking-tight'>Dashboard</h1>
            <p className='text-muted-foreground text-sm'>
              {site ? `${site.display_name} · live overview` : 'Loading site…'}
            </p>
          </div>
        </div>

        <div className='space-y-4'>
          <KpiRow siteId={site?.id} />
          <div className='grid gap-4 lg:grid-cols-3'>
            <DemandChart siteId={site?.id} />
            <RecentSparks siteId={site?.id} />
          </div>
          <div className='grid gap-4 lg:grid-cols-3'>
            <EquipmentHealth siteId={site?.id} />
          </div>
        </div>
      </Main>
    </>
  )
}
