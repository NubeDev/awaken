/**
 * Binds the time store to the URL and runs the auto-refresh loop
 * (docs/design/time-range-and-refresh.md §§2-3,5). On a store change it writes
 * `?from/to/refresh` (replaceState, no history spam) and notifies same-tab
 * listeners — the same mechanism the variable URL state uses. While a refresh
 * interval is set it bumps the store `tick` on a timer, **pausing when the tab is
 * hidden** so a backgrounded board does not poll, and re-freezes `now` per tick.
 *
 * Mount once near the dashboard root. Widgets read `tick`/`from`/`to` from the
 * store and fold them into their query keys; this hook owns the side effects.
 */
import { useEffect } from 'react'
import { useTimeStore } from '@/stores/time-store'
import { writeTimeParams } from './url-state'

const TIME_URL_EVENT = 'rubix:time-url-change'

export function useTimeRangeSync(): void {
  const from = useTimeStore((s) => s.from)
  const to = useTimeStore((s) => s.to)
  const refresh = useTimeStore((s) => s.refresh)
  const bumpTick = useTimeStore((s) => s.bumpTick)

  // Reflect the current selection into the URL whenever it changes.
  useEffect(() => {
    const base = new URLSearchParams(window.location.search)
    const next = writeTimeParams(base, { from, to, refresh })
    const qs = next.toString()
    const url = `${window.location.pathname}${qs ? `?${qs}` : ''}${window.location.hash}`
    window.history.replaceState(window.history.state, '', url)
    window.dispatchEvent(new Event(TIME_URL_EVENT))
  }, [from, to, refresh])

  // Auto-refresh timer: tick at `refresh` seconds, paused while the tab is
  // hidden. A bump when the tab regains focus catches up a stale board.
  useEffect(() => {
    if (refresh <= 0) return
    let timer: ReturnType<typeof setInterval> | undefined

    const start = () => {
      if (timer !== undefined) return
      timer = setInterval(() => bumpTick(), refresh * 1000)
    }
    const stop = () => {
      if (timer === undefined) return
      clearInterval(timer)
      timer = undefined
    }
    const onVisibility = () => {
      if (document.hidden) {
        stop()
      } else {
        bumpTick() // catch up the missed refresh on return
        start()
      }
    }

    if (!document.hidden) start()
    document.addEventListener('visibilitychange', onVisibility)
    return () => {
      stop()
      document.removeEventListener('visibilitychange', onVisibility)
    }
  }, [refresh, bumpTick])
}
