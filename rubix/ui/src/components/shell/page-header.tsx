// A tiny context so each page can feed its breadcrumbs / live-point count / site
// name up into the one shared AppHeader, instead of every page rendering its own
// full-width TopBar. A page calls usePageHeader({...}) once; the header reads the
// current value. Set values are cleared on unmount so stale crumbs don't linger.

import { createContext, useContext, useEffect, useState } from 'react'

export interface PageHeaderState {
  crumbs?: string[]
  livePoints?: number
  /** Active site key + display name, shown in the header's site chip. */
  site?: string
  siteName?: string
}

interface PageHeaderContextValue {
  header: PageHeaderState
  setHeader: (h: PageHeaderState) => void
}

const PageHeaderContext = createContext<PageHeaderContextValue | null>(null)

export function PageHeaderProvider({ children }: { children: React.ReactNode }) {
  const [header, setHeader] = useState<PageHeaderState>({})
  return <PageHeaderContext.Provider value={{ header, setHeader }}>{children}</PageHeaderContext.Provider>
}

// Read-side, for AppHeader.
export function usePageHeaderState(): PageHeaderState {
  const ctx = useContext(PageHeaderContext)
  return ctx?.header ?? {}
}

// Write-side, for pages. Re-runs whenever the passed values change; clears on
// unmount. The deps are spread so a page can pass a fresh object inline.
export function usePageHeader(state: PageHeaderState) {
  const ctx = useContext(PageHeaderContext)
  const setHeader = ctx?.setHeader
  const { crumbs, livePoints, site, siteName } = state
  const crumbsKey = crumbs?.join('›')
  useEffect(() => {
    setHeader?.({ crumbs, livePoints, site, siteName })
    return () => setHeader?.({})
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [setHeader, crumbsKey, livePoints, site, siteName])
}
