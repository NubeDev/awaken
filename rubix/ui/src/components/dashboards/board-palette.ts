// The builder palette model: the draggable items the left rail offers and the
// drag payload they carry. An item is either a one-click preset (materialised into
// a kind:"chart" record on drop) or an already-saved chart (placed directly). The
// drag transfers a small JSON token via the dataTransfer; the grid decodes it on
// drop and asks the page to add the panel.

import {
  Activity,
  BarChart3,
  BarChartHorizontal,
  LineChart as LineChartIcon,
  type LucideIcon,
} from 'lucide-react'
import { ChartType } from '../chart-builder/types'
import type { ChartConfig } from '../chart-builder/types'

/** The MIME-ish key under which a dragged palette item rides the dataTransfer. */
export const PALETTE_DND_TYPE = 'application/x-rubix-palette'

/** What a drag carries: either an existing chart id, or a preset to materialise. */
export type PaletteDrag =
  | { source: 'chart'; chartId: string }
  | { source: 'preset'; preset: string }

export function encodeDrag(d: PaletteDrag): string {
  return JSON.stringify(d)
}

export function decodeDrag(raw: string): PaletteDrag | null {
  try {
    const v = JSON.parse(raw) as PaletteDrag
    if (v && (v.source === 'chart' || v.source === 'preset')) return v
  } catch {
    /* not our payload */
  }
  return null
}

/** Pick a representative icon for a chart config so the palette reads at a glance. */
export function chartIcon(config: ChartConfig | undefined): LucideIcon {
  switch (config?.type) {
    case ChartType.LineChart:
    case ChartType.AreaChart:
      return LineChartIcon
    case ChartType.HorizontalBarChart:
      return BarChartHorizontal
    case ChartType.BarChart:
      return BarChart3
    default:
      return Activity
  }
}
