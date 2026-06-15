// Shared helpers the vendored Laminar chart components import from `@/lib/utils`.
// `cn` re-exports the existing class-merge helper (one source of truth, see
// `@/lib/cn`); the two formatters are ported from Laminar's `lib/utils.ts` (the
// only ones the chart layer needs — relative-time labels and a seconds→duration
// formatter for cost/duration columns), trimmed of their Next/server context.

export { cn } from '@/lib/cn'

/** A compact "3m ago" / "2d ago" relative-time label; "Never" for null. */
export function formatRelativeTime(dateStr: string | null): string {
  if (!dateStr) return 'Never'
  const date = new Date(dateStr)
  const now = new Date()
  const diffMs = now.getTime() - date.getTime()
  const diffMins = Math.floor(diffMs / (1000 * 60))
  const diffHours = Math.floor(diffMs / (1000 * 60 * 60))
  const diffDays = Math.floor(diffMs / (1000 * 60 * 60 * 24))

  if (diffMins < 1) return 'Just now'
  if (diffMins < 60) return `${diffMins}m ago`
  if (diffHours < 24) return `${diffHours}h ago`
  if (diffDays < 30) return `${diffDays}d ago`
  return date.toLocaleDateString()
}

/** Render a seconds value as `1h 2m 3s`, dropping zero leading units. */
export function formatSecsToHoursMinsSecs(seconds: number): string {
  let h = Math.floor(seconds / 3600)
  let m = Math.floor((seconds % 3600) / 60)
  let s = seconds % 60

  const precision = s < 1 ? 2 : 1
  const rounded = parseFloat(s.toFixed(precision))
  if (rounded >= 60) {
    s = 0
    m += 1
    if (m >= 60) {
      m = 0
      h += 1
    }
  } else {
    s = rounded
  }

  const parts: string[] = []
  if (h > 0) parts.push(`${h}h`)
  if (m > 0) parts.push(`${m}m`)
  if (parts.length === 0 || s > 0) parts.push(`${s}s`)
  return parts.join(' ')
}
