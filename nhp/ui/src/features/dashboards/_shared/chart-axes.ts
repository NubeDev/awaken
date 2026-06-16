/**
 * Shared cartesian-chart chrome constants for the recharts trend widgets — themed
 * axis/grid/margin props in ONE place so line/area/bar render identically and track
 * the app theme (--border/--muted-foreground) instead of recharts' stark defaults.
 * The themed tooltip COMPONENT lives in chart-theme.tsx (kept separate so this file
 * is constants-only). DASHBOARDS-SCOPE §7: palette via theme.
 */

/** Axis tick label styling shared by every cartesian widget. */
export const AXIS_TICK = { fontSize: 11, fill: 'var(--muted-foreground)' } as const

/** Axis baseline stroke (only the x-axis draws one; y-axis is axis-line-free). */
export const AXIS_LINE = { stroke: 'var(--border)' } as const

/** A soft, theme-aware grid — horizontal-only reads cleaner for a trend. */
export const GRID_PROPS = {
  strokeDasharray: '3 3',
  stroke: 'var(--border)',
  vertical: false,
} as const

/** The shared chart margin — a touch of right padding so the last tick isn't clipped. */
export const CHART_MARGIN = { top: 8, right: 12, bottom: 4, left: 0 } as const
