// The one shell wrapping every tenant screen (operator + admin): the floating
// inset sidebar, the sticky header, and the routed content. Mounted once by the
// `/t/$tenant` route, so screens no longer carry their own chrome. Reads the
// persisted sidebar-open cookie so the collapsed/expanded state survives reloads.

import { Outlet, useParams } from '@tanstack/react-router'
import { SidebarInset, SidebarProvider } from '@/components/ui/sidebar'
import { AppSidebar } from './AppSidebar'
import { AppHeader } from './AppHeader'
import { PageHeaderProvider } from './page-header'

function readSidebarCookie(): boolean {
  if (typeof document === 'undefined') return true
  return !document.cookie.split('; ').some((c) => c === 'sidebar_state=false')
}

export function AppShell() {
  const { tenant } = useParams({ from: '/t/$tenant' })
  return (
    <PageHeaderProvider>
      <SidebarProvider defaultOpen={readSidebarCookie()}>
        <AppSidebar />
        <SidebarInset className="flex h-svh min-h-0 flex-col overflow-hidden">
          <AppHeader tenant={tenant} />
          <div className="min-h-0 flex-1 overflow-auto">
            <Outlet />
          </div>
        </SidebarInset>
      </SidebarProvider>
    </PageHeaderProvider>
  )
}
