import { Header } from '@/components/layout/header'
import { Main } from '@/components/layout/main'
import { ConfigDrawer } from '@/components/config-drawer'
import { ProfileDropdown } from '@/components/profile-dropdown'
import { ThemeSwitch } from '@/components/theme-switch'

/**
 * App-shell placeholder for a feature a later WS owns. Renders the standard
 * NHP chrome (header + main) with a title and the WS that will fill it, so the
 * shell is navigable end-to-end before features land. See WS-01.md.
 */
export function PlaceholderPage({
  title,
  owner,
}: {
  title: string
  owner: string
}) {
  return (
    <>
      <Header>
        <h1 className='me-auto text-base font-medium'>{title}</h1>
        <ThemeSwitch />
        <ConfigDrawer />
        <ProfileDropdown />
      </Header>
      <Main>
        <div className='flex h-full flex-col items-center justify-center gap-2 text-center'>
          <h2 className='text-2xl font-semibold tracking-tight'>{title}</h2>
          <p className='text-muted-foreground max-w-md text-sm'>
            This area is part of the NHP app shell. It will be built by {owner}.
          </p>
        </div>
      </Main>
    </>
  )
}
