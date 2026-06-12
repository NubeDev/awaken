import { create } from 'zustand'
import { persist } from 'zustand/middleware'
import type { Uuid } from '@/api/types'

/**
 * The operator's currently-selected site. Persisted so a reload keeps context.
 * Pages read `siteId` to scope their API queries; the site switcher sets it.
 * `null` means "not chosen yet" — consumers fall back to the first site loaded.
 */
type SiteState = {
  siteId: Uuid | null
  setSite: (id: Uuid) => void
}

export const useSiteStore = create<SiteState>()(
  persist(
    (set) => ({
      siteId: null,
      setSite: (id) => set({ siteId: id }),
    }),
    { name: 'rubix.site' }
  )
)
