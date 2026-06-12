import { ConfigDrawer } from '@/components/config-drawer'
import { ProfileDropdown } from '@/components/profile-dropdown'
import { Search } from '@/components/search'
import { ThemeSwitch } from '@/components/theme-switch'
import { Header } from './header'
import { PageTitle } from './page-title'

/**
 * Standard top chrome for every surface: page title + live dot on the left,
 * ask-awaken search and utilities on the right — the mockup's header layout.
 */
export function PageHeader({ title, sub }: { title: string; sub?: string }) {
  return (
    <Header fixed>
      <PageTitle title={title} sub={sub} />
      <div className='ms-auto flex items-center gap-2'>
        <Search />
        <ThemeSwitch />
        <ConfigDrawer />
        <ProfileDropdown />
      </div>
    </Header>
  )
}
