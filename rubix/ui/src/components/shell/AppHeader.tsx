// The one shared sticky header: sidebar trigger, the active site chip, the page's
// breadcrumbs and live-point count (fed up via PageHeader context), a search
// affordance and a quick light/dark toggle. Replaces the per-screen TopBar.

import { Link } from '@tanstack/react-router'
import { Building2, ChevronRight, ChevronsUpDown, Moon, Search, Sun } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Separator } from '@/components/ui/separator'
import { SidebarTrigger } from '@/components/ui/sidebar'
import { useTheme } from '@/context/theme-provider'
import { usePageHeaderState } from './page-header'

export function AppHeader({ tenant }: { tenant: string }) {
  const { crumbs, livePoints, site, siteName } = usePageHeaderState()
  const { resolvedTheme, setTheme } = useTheme()

  return (
    <header className="sticky top-0 z-40 flex h-14 shrink-0 items-center gap-2 border-b border-border bg-background/80 px-4 backdrop-blur">
      <SidebarTrigger className="-ml-1" />
      <Separator orientation="vertical" className="mr-1 h-4" />

      <Link
        to="/"
        search={site ? { site } : {}}
        className="flex items-center gap-2 text-[13px] text-muted transition hover:text-fg"
      >
        <Building2 size={16} />
        <span className="font-medium text-fg">{siteName ?? tenant}</span>
        <ChevronsUpDown size={14} />
      </Link>

      {crumbs && crumbs.length > 0 && (
        <>
          <ChevronRight className="text-muted" size={14} />
          <div className="flex items-center gap-2 text-[13px] text-muted">
            {crumbs.map((cr, i) => (
              <span key={cr} className={i === crumbs.length - 1 ? 'font-medium text-fg' : ''}>
                {cr}
              </span>
            ))}
          </div>
        </>
      )}

      <div className="ml-auto flex items-center gap-3">
        {livePoints != null && (
          <span className="mono flex items-center gap-1.5 text-[12px] text-muted">
            <span className="size-1.5 rounded-full bg-green blink" />
            {livePoints} live
          </span>
        )}
        <Button
          type="button"
          variant="outline"
          className="h-8 px-2.5 text-[12px] font-normal text-muted hover:text-fg"
        >
          <Search size={14} />
          Search<kbd>⌘K</kbd>
        </Button>
        <Button
          type="button"
          variant="ghost"
          size="icon"
          className="size-8 text-muted hover:text-fg"
          aria-label="Toggle theme"
          onClick={() => setTheme(resolvedTheme === 'dark' ? 'light' : 'dark')}
        >
          {resolvedTheme === 'dark' ? <Sun size={16} /> : <Moon size={16} />}
        </Button>
      </div>
    </header>
  )
}
