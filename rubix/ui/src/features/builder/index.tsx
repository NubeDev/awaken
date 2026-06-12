import { LayoutGrid } from 'lucide-react'
import { ConfigDrawer } from '@/components/config-drawer'
import { Header } from '@/components/layout/header'
import { Main } from '@/components/layout/main'
import { ProfileDropdown } from '@/components/profile-dropdown'
import { Search } from '@/components/search'
import { ThemeSwitch } from '@/components/theme-switch'

/**
 * Dashboard Builder surface. Widget binding writes to `/api/v1/widgets`; the
 * compose UI is not built yet, so this states the surface honestly rather than
 * rendering fabricated saved dashboards.
 */
export function Builder() {
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
        <div className='grid h-full place-items-center'>
          <div className='max-w-sm text-center'>
            <div className='bg-accent text-primary mx-auto mb-4 grid size-14 place-items-center rounded-xl'>
              <LayoutGrid className='size-6' />
            </div>
            <h2 className='mb-2 text-lg font-semibold'>Dashboard Builder</h2>
            <p className='text-muted-foreground text-sm leading-relaxed'>
              Compose and bind widgets to live points. The widget store is wired at{' '}
              <code className='font-mono text-[12px]'>/api/v1/widgets</code>; the drag-and-drop
              composer lands next.
            </p>
          </div>
        </div>
      </Main>
    </>
  )
}
