import type { PointValue } from '@/api/types'

/** Render a point value for display, formatting numbers to a sane precision. */
export function formatValue(value: PointValue | null | undefined, unit?: string | null): string {
  if (value === null || value === undefined) return '—'
  if (typeof value === 'boolean') return value ? 'On' : 'Off'
  if (typeof value === 'number') {
    const n = Math.abs(value) >= 100 ? Math.round(value) : Math.round(value * 10) / 10
    return unit ? `${n}${unit === '%' || unit === '°C' ? '' : ' '}${unit}` : String(n)
  }
  return value
}

/** Ultra-short age from an ISO timestamp, e.g. "12s", "4m", "3h". */
export function ageShort(iso: string | null | undefined, now = Date.now()): string {
  const rel = relativeTime(iso, now)
  return rel === '—' ? rel : rel.replace(' ago', '')
}

/** Compact relative time from an ISO timestamp, e.g. "6m ago". */
export function relativeTime(iso: string | null | undefined, now = Date.now()): string {
  if (!iso) return '—'
  const then = new Date(iso).getTime()
  if (Number.isNaN(then)) return '—'
  const secs = Math.max(0, Math.round((now - then) / 1000))
  if (secs < 60) return `${secs}s ago`
  const mins = Math.round(secs / 60)
  if (mins < 60) return `${mins}m ago`
  const hrs = Math.round(mins / 60)
  if (hrs < 24) return `${hrs}h ago`
  return `${Math.round(hrs / 24)}d ago`
}
