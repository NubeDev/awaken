/**
 * Sync variable selections to the URL `?var-*` query string
 * (docs/design/variables-and-templating.md §7), so a selection deep-links and a
 * shared link restores. Rubix's typed route search schemas would strip unknown
 * params, so this reads/writes `window.location` directly and replaces history
 * (no new entry per selection) rather than threading through the router's typed
 * search. The `var-` prefix is reserved for explicit variable state; bare params
 * belong to the page-context source (WS-06).
 */
import { useCallback, useEffect, useState } from 'react'
import type { VariableValue } from '@/api/types'
import { readVarParams, writeVarParams } from './url-state'

/** A same-tab event so multiple readers of the var state stay in sync without a
 *  full navigation (popstate only fires on back/forward). */
const VAR_URL_EVENT = 'rubix:var-url-change'

function currentParams(): URLSearchParams {
  return new URLSearchParams(window.location.search)
}

/**
 * The current `?var-*` selection and a setter that writes it back to the URL.
 * The setter replaces the variable params (preserving any non-`var-` params) and
 * notifies same-tab listeners; it never pushes a history entry.
 */
export function useVarUrlState(): {
  selection: Record<string, VariableValue>
  setSelection: (name: string, value: VariableValue) => void
} {
  const [selection, setStateSelection] = useState<
    Record<string, VariableValue>
  >(() => readVarParams(currentParams()))

  useEffect(() => {
    const sync = () => setStateSelection(readVarParams(currentParams()))
    window.addEventListener('popstate', sync)
    window.addEventListener(VAR_URL_EVENT, sync)
    return () => {
      window.removeEventListener('popstate', sync)
      window.removeEventListener(VAR_URL_EVENT, sync)
    }
  }, [])

  const setSelection = useCallback(
    (name: string, value: VariableValue) => {
      const base = currentParams()
      const merged = { ...readVarParams(base), [name]: value }
      const next = writeVarParams(base, merged)
      const qs = next.toString()
      const url = `${window.location.pathname}${qs ? `?${qs}` : ''}${window.location.hash}`
      window.history.replaceState(window.history.state, '', url)
      window.dispatchEvent(new Event(VAR_URL_EVENT))
    },
    []
  )

  return { selection, setSelection }
}
