// The product top bar — orb/home link, tenant·site switcher, breadcrumbs, a live
// point count, search affordance and the avatar. Ported from the demo's
// screens.js `topbar()`, navigation rewired to TanStack Router.

import { Link } from '@tanstack/react-router'
import { Building2, ChevronRight, ChevronsUpDown, Search, SlidersHorizontal } from 'lucide-react'
import { Orb } from './Orb'
import { Button } from './button'

interface TopBarProps {
  tenant: string
  site?: string
  siteName?: string
  crumbs?: string[]
  livePoints?: number
}

export function TopBar({ tenant, site, siteName, crumbs, livePoints }: TopBarProps) {
  const homeSearch = site ? { site } : {}
  return (
    <header className="h-14 shrink-0 flex items-center gap-3 px-6 border-b border-border">
      <Link to="/t/$tenant" params={{ tenant }} search={homeSearch} className="flex items-center gap-2.5 group">
        <Orb size={28} blur />
        <span className="font-semibold tracking-tight">Rubix</span>
      </Link>
      <div className="h-4 w-px bg-border mx-1" />
      <Link to="/" className="flex items-center gap-2 text-[13px] hover:text-fg transition text-muted">
        <Building2 className="text-muted" size={16} />
        <span className="font-medium text-fg">{siteName ?? tenant}</span>
        <ChevronsUpDown className="text-muted" size={14} />
      </Link>
      {crumbs && crumbs.length > 0 && (
        <>
          <ChevronRight className="text-muted" size={14} />
          <div className="flex items-center gap-2 text-[13px] text-muted">
            {crumbs.map((cr, i) => (
              <span key={cr} className={i === crumbs.length - 1 ? 'text-fg font-medium' : ''}>
                {cr}
              </span>
            ))}
          </div>
        </>
      )}
      <div className="ml-auto flex items-center gap-5 text-[12px] mono text-muted">
        {livePoints != null && (
          <span className="flex items-center gap-1.5">
            <span className="size-1.5 rounded-full bg-green blink" />
            {livePoints} live
          </span>
        )}
      </div>
      <div className="h-4 w-px bg-border mx-1" />
      {/* Admin console — always reachable, from any screen. Highlights when the
          console is the active route (data-status="active" from TanStack Router). */}
      <Link
        to="/t/$tenant/admin"
        params={{ tenant }}
        className="flex items-center gap-1.5 h-8 px-2.5 rounded-lg border border-border text-[12px] text-muted hover:text-fg hover:bg-panel2 transition data-[status=active]:text-fg data-[status=active]:border-r1/40 data-[status=active]:bg-r1/10"
      >
        <SlidersHorizontal size={14} />
        Admin
      </Link>
      <Button
        type="button"
        variant="outline"
        className="h-8 px-2.5 text-[12px] font-normal text-muted hover:text-fg hover:bg-panel2"
      >
        <Search size={14} />
        Search<kbd>⌘K</kbd>
      </Button>
      <div className="size-8 rounded-full bg-panel2 border border-border grid place-items-center text-xs font-semibold">
        AK
      </div>
    </header>
  )
}
