/**
 * A tiny slot bridge so a PAGE can render content into the app's persistent top
 * Header (authenticated-layout.tsx) — the shadcn-admin convention is the
 * breadcrumb/nav living in the top bar, but NHP has ONE global header (the
 * SidebarTrigger must be on every page, WS-01) rather than a per-page header.
 *
 * The dashboard page owns its drill-stack breadcrumb as local state; it sets that
 * node here on mount/update, and the layout Header renders whatever is set (or
 * nothing). Mirrors the layout-provider pattern (context + hook). Set to null on
 * unmount so the breadcrumb doesn't leak onto other pages.
 */
import {
  createContext,
  useContext,
  useEffect,
  useState,
  type ReactNode,
} from 'react'

type HeaderSlotContextType = {
  left: ReactNode
  setLeft: (node: ReactNode) => void
}

const HeaderSlotContext = createContext<HeaderSlotContextType | null>(null)

export function HeaderSlotProvider({ children }: { children: ReactNode }) {
  const [left, setLeft] = useState<ReactNode>(null)
  return (
    <HeaderSlotContext value={{ left, setLeft }}>{children}</HeaderSlotContext>
  )
}

// eslint-disable-next-line react-refresh/only-export-components
export function useHeaderSlot() {
  const ctx = useContext(HeaderSlotContext)
  if (!ctx) throw new Error('useHeaderSlot must be used within a HeaderSlotProvider')
  return ctx
}

/**
 * Render `node` into the header's left slot for as long as this hook's owner is
 * mounted. `node` is re-published whenever it changes (so a live breadcrumb
 * updates), and cleared on unmount.
 */
// eslint-disable-next-line react-refresh/only-export-components
export function useHeaderLeft(node: ReactNode) {
  const { setLeft } = useHeaderSlot()
  useEffect(() => {
    setLeft(node)
    return () => setLeft(null)
  }, [node, setLeft])
}
