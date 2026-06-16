// The widget renderMap (§8) — type → renderer, the dispatch half of the catalog
// discipline. Each entry is a pure function of one shared render context, so the
// renderer (`ChartRendererCore`) never grows a per-type switch: it builds the
// context once, validates against the catalog's roles, and looks the renderer up
// here. Adding a widget is one catalog entry + one entry here.

import type { ReactNode } from 'react'

import AreaChart from './area-chart'
import BarChart from './bar-chart'
import HorizontalBarChart from './horizontal-bar-chart'
import LineChart, { type ChartDragHandlers } from './line-chart'
import PieChart from './pie-chart'
import TableChart from './table-chart'
import { ChartType, type ChartConfig as ChartBuilderConfig, resolveDisplayMode } from '../types'
import type { ColumnInfo } from '../utils'
import type { ChartConfig as UiChartConfig } from '@/components/ui/chart'

/** Everything a widget renderer might need, built once by `ChartRendererCore`.
 *  A renderer reads only the slice it cares about (table → columns; pie → raw
 *  rows; cartesian → grouped series). */
export interface WidgetRenderContext {
  config: ChartBuilderConfig
  /** The raw result rows (pie/table read these directly). */
  data: Record<string, any>[]
  /** The breakdown-grouped rows for cartesian series. */
  chartData: Record<string, any>[]
  /** The series keys after grouping (one per line/bar/area series). */
  keys: string[]
  /** The recharts series config (colour/label per key). */
  uiChartConfig: UiChartConfig
  columns: ColumnInfo[]
  hiddenColumns?: string[]
  onBarClick?: (rowData: Record<string, any>) => void
  onColumnConfigChange?: (config: import('../types').TableColumnConfig) => void
  syncId?: string
  drag?: ChartDragHandlers
  hasMore?: boolean
  isFetching?: boolean
  fetchNextPage?: () => void
}

/** The shared cartesian props (line/area/bar) derived from a context. */
function cartesianProps(ctx: WidgetRenderContext) {
  const { config } = ctx
  return {
    data: ctx.chartData,
    x: config.x!,
    y: config.y!,
    breakdown: config.breakdown,
    displayMode: resolveDisplayMode(config),
    metricColumn: config.type === ChartType.HorizontalBarChart ? config.x : config.y,
    keys: ctx.keys,
    chartConfig: ctx.uiChartConfig,
    fieldConfig: config.fieldConfig,
    syncId: ctx.syncId,
    drag: ctx.drag,
  }
}

/** type → renderer. Keyed by `ChartType` so the compiler forces an entry for
 *  every widget the catalog declares. */
export const RENDER_MAP: Record<ChartType, (ctx: WidgetRenderContext) => ReactNode> = {
  [ChartType.LineChart]: (ctx) => <LineChart {...cartesianProps(ctx)} />,
  [ChartType.AreaChart]: (ctx) => <AreaChart {...cartesianProps(ctx)} />,
  [ChartType.BarChart]: (ctx) => <BarChart {...cartesianProps(ctx)} />,
  [ChartType.HorizontalBarChart]: (ctx) => {
    const { syncId: _s, drag: _d, ...rest } = cartesianProps(ctx)
    return <HorizontalBarChart {...rest} onBarClick={ctx.onBarClick} />
  },
  [ChartType.PieChart]: (ctx) => (
    <PieChart data={ctx.data} x={ctx.config.x!} y={ctx.config.y!} fieldConfig={ctx.config.fieldConfig} />
  ),
  [ChartType.Table]: (ctx) => (
    <TableChart
      data={ctx.data}
      columns={ctx.columns}
      hiddenColumns={ctx.hiddenColumns}
      onRowClick={ctx.onBarClick}
      tableColumnConfig={ctx.config.type === ChartType.Table ? ctx.config.tableColumnConfig : undefined}
      onColumnConfigChange={ctx.onColumnConfigChange}
      hasMore={ctx.hasMore}
      isFetching={ctx.isFetching}
      fetchNextPage={ctx.fetchNextPage}
    />
  ),
}
