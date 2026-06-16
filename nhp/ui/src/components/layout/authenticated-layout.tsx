import { Outlet } from '@tanstack/react-router'
import { getCookie } from '@/lib/cookies'
import { cn } from '@/lib/utils'
import { LayoutProvider } from '@/context/layout-provider'
import { HeaderSlotProvider, useHeaderSlot } from '@/context/header-slot'
import { SidebarInset, SidebarProvider } from '@/components/ui/sidebar'
import { AppSidebar } from '@/components/layout/app-sidebar'
import { Header } from '@/components/layout/header'
import { SkipToMain } from '@/components/skip-to-main'
import { ConfigDrawer } from '@/components/config-drawer'
import { ProfileDropdown } from '@/components/profile-dropdown'
import { ThemeSwitch } from '@/components/theme-switch'

type AuthenticatedLayoutProps = {
  children?: React.ReactNode
}

/**
 * NHP app shell. Dropped rubix-old's ScopeProvider (org/site URL scope) and
 * SearchProvider/command-menu (coupled to the ask-awaken AI feature) — NHP's
 * nav is flat and those domains are out of scope. See WS-01.md.
 */
export function AuthenticatedLayout({ children }: AuthenticatedLayoutProps) {
  const defaultOpen = getCookie('sidebar_state') !== 'false'
  return (
    <LayoutProvider>
      <HeaderSlotProvider>
        <SidebarProvider defaultOpen={defaultOpen}>
          <SkipToMain />
          <AppSidebar />
          <SidebarInset
            className={cn(
              // Set content container, so we can use container queries
              '@container/content',
              // If layout is fixed, set the height to 100svh to prevent overflow
              'has-data-[layout=fixed]:h-svh',
              // If layout is fixed and sidebar is inset, subtract the margins
              'peer-data-[variant=inset]:has-data-[layout=fixed]:h-[calc(100svh-(var(--spacing)*4))]'
            )}
          >
            {/* Persistent header so the SidebarTrigger is on every page — without
                it a collapsed sidebar can't be reopened (feature pages render no
                header of their own). See nhp/docs/sessions/WS-01.md. The left slot
                carries a page-published breadcrumb (shadcn-admin top-bar nav). */}
            <Header fixed>
              <HeaderLeftSlot />
              <div className='ms-auto flex items-center gap-2'>
                <ThemeSwitch />
                <ConfigDrawer />
                <ProfileDropdown />
              </div>
            </Header>
            {children ?? <Outlet />}
          </SidebarInset>
        </SidebarProvider>
      </HeaderSlotProvider>
    </LayoutProvider>
  )
}

/** Renders whatever the current page published into the header's left slot. */
function HeaderLeftSlot() {
  const { left } = useHeaderSlot()
  return <>{left}</>
}
