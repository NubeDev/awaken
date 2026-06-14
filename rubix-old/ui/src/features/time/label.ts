/**
 * Human label for the current `{from, to}` selection shown on the picker trigger
 * (docs/design/time-range-and-refresh.md §2). A range matching a quick preset
 * shows that preset's label; otherwise it shows the raw tokens (relative) or
 * localised instants (absolute). Pure, so the picker and tests share it.
 */
import { QUICK_RANGES } from './presets'

function isAbsolute(token: string): boolean {
  return !token.trim().startsWith('now')
}

/** A short localised label for an absolute instant, else the relative token. */
function boundLabel(token: string): string {
  if (!isAbsolute(token)) return token
  const ms = Date.parse(token)
  if (Number.isNaN(ms)) return token
  return new Date(ms).toLocaleString([], {
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  })
}

export function rangeLabel(from: string, to: string): string {
  const preset = QUICK_RANGES.find((q) => q.from === from && q.to === to)
  if (preset) return preset.label
  return `${boundLabel(from)} → ${boundLabel(to)}`
}
