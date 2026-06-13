/**
 * The chart types a `point_history` tile can render, used by the binder picker.
 * Kept apart from the chart components in `charts.tsx` so the registry is a
 * plain-data export (no React-refresh component-export warning).
 */
import {
  AreaChart as AreaIcon,
  BarChart3,
  LineChart as LineIcon,
  Table as TableIcon,
  type LucideIcon,
} from 'lucide-react'
import type { ChartType } from '@/api/types'

export const CHART_TYPES: {
  type: ChartType
  label: string
  icon: LucideIcon
}[] = [
  { type: 'area', label: 'Area', icon: AreaIcon },
  { type: 'line', label: 'Line', icon: LineIcon },
  { type: 'bar', label: 'Bar', icon: BarChart3 },
  { type: 'table', label: 'Table', icon: TableIcon },
]
