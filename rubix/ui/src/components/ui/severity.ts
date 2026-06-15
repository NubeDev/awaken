// Severity → colour token + label, ported from the demo's RX.sevMap.

import type { Severity } from '../../types/Domain'

export const sevMap: Record<Severity, { c: string; label: string }> = {
  crit: { c: 'crit', label: 'Critical' },
  amber: { c: 'amber', label: 'Needs action' },
  green: { c: 'green', label: 'Good' },
  muted: { c: 'muted', label: 'Watching' },
}
