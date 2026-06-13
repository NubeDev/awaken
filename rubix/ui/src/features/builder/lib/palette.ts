/**
 * The builder palette: each entry maps a tile to a real `WidgetKind` or marks
 * itself unavailable. Only the three kinds the server persists
 * (`crates/rubix-core` `WidgetKind`) are bindable; the mockup also showed
 * Gauge / Floor Map tiles with no wire backing, so they appear disabled with a
 * reason rather than pretending to be clickable.
 */
import {
  Activity,
  Gauge,
  LineChart,
  Map,
  SquareStack,
  type LucideIcon,
} from 'lucide-react'
import type { WidgetKind } from '@/api/types'

/** How a bindable palette entry sources its target. */
export type BindMode = 'point' | 'board'

export type PaletteEntry = {
  label: string
  description: string
  icon: LucideIcon
} & (
  | { kind: WidgetKind; bind: BindMode; available: true }
  | { available: false; reason: string }
)

export const PALETTE: PaletteEntry[] = [
  {
    label: 'Live Value',
    description: 'Current value of a point with freshness.',
    icon: Activity,
    kind: 'point_value',
    bind: 'point',
    available: true,
  },
  {
    label: 'Line / Area',
    description: 'Time-series history of a point.',
    icon: LineChart,
    kind: 'point_history',
    bind: 'point',
    available: true,
  },
  {
    label: 'Board Output',
    description: 'Latest run output of a stored board.',
    icon: SquareStack,
    kind: 'board_output',
    bind: 'board',
    available: true,
  },
  {
    label: 'Gauge',
    description: 'Radial gauge tile.',
    icon: Gauge,
    available: false,
    reason: 'No widget backing yet — pin a Live Value instead.',
  },
  {
    label: 'Floor Map',
    description: 'Spatial floor overlay.',
    icon: Map,
    available: false,
    reason: 'No widget backing yet.',
  },
]
