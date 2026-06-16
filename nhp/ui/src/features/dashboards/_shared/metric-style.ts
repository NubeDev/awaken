/**
 * Semantic metric vocabulary (the "more colour" half of WS-07 polish): a stable
 * mapping from a register's `quantity` to a chart colour, an accent and a lucide
 * icon, so a power tile, a power chart and a power KPI all read the SAME colour
 * across every level of the board. This is presentation-only — it does NOT touch
 * the alarm severity ramp (palette.ts SEVERITY_COLORS), which stays semantic and
 * always wins over a metric accent when a value is in alarm.
 *
 * Colours reference the theme's `--chart-N` vars (track light/dark like the
 * series palette) but are pinned PER QUANTITY rather than cycled by index, so the
 * accent is deterministic for a given metric instead of dependent on render order.
 */
import {
  Activity,
  Gauge,
  Plug,
  Sigma,
  Waves,
  Zap,
  type LucideIcon,
} from 'lucide-react'

export interface MetricStyle {
  /** Theme chart var, e.g. 'var(--chart-1)'. */
  color: string
  /** lucide icon for tiles/KPIs. */
  icon: LucideIcon
}

/**
 * Quantity → style. Keys are the seeded `quantity` tag values (DOMAIN-MODEL). An
 * unknown quantity falls back to a neutral accent so a new metric still renders.
 */
const BY_QUANTITY: Record<string, MetricStyle> = {
  power: { color: 'var(--chart-1)', icon: Zap },
  energy: { color: 'var(--chart-5)', icon: Activity },
  voltage: { color: 'var(--chart-4)', icon: Plug },
  current: { color: 'var(--chart-3)', icon: Waves },
  frequency: { color: 'var(--chart-2)', icon: Gauge },
  power_factor: { color: 'var(--chart-2)', icon: Sigma },
}

const FALLBACK: MetricStyle = { color: 'var(--chart-1)', icon: Activity }

export function metricStyle(quantity: string | undefined): MetricStyle {
  if (!quantity) return FALLBACK
  return BY_QUANTITY[quantity] ?? FALLBACK
}
