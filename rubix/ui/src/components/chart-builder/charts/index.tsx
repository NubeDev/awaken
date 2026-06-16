import { useMemo } from 'react'

import { type ChartDragHandlers } from '@/components/chart-builder/charts/line-chart'
import {
  generateChartConfig,
  transformDataForBreakdown,
  transformDataForSimpleChart,
} from '@/components/chart-builder/charts/utils'
import { RENDER_MAP, type WidgetRenderContext } from '@/components/chart-builder/charts/render-map'
import {
  type ChartConfig,
  ChartType,
  type TableColumnConfig,
} from '@/components/chart-builder/types'
import { allowsBreakdown, descriptor, missingFields, needsX } from '@/components/chart-builder/catalog'
import { type ColumnInfo } from '@/components/chart-builder/utils'

// Vendored from Laminar's `chart-builder/charts/index.tsx`. The store-coupled
// default `ChartRenderer` is dropped — `ChartRendererCore` is the reusable,
// store-free renderer (config + rows + columns → chart). The builder UI + store
// land in a later slice; the SQL console / dashboards drive this directly.

interface ChartRendererCoreProps {
  config: ChartConfig
  data: Record<string, any>[]
  columns: ColumnInfo[]
  hiddenColumns?: string[]
  onBarClick?: (rowData: Record<string, any>) => void
  syncId?: string
  drag?: ChartDragHandlers
  onColumnConfigChange?: (config: TableColumnConfig) => void
  hasMore?: boolean
  isFetching?: boolean
  fetchNextPage?: () => void
}

export const ChartRendererCore = ({
  config,
  data,
  columns,
  hiddenColumns,
  onBarClick,
  syncId,
  drag,
  onColumnConfigChange,
  hasMore,
  isFetching,
  fetchNextPage,
}: ChartRendererCoreProps) => {
  // Grouped cartesian series — only computed for widgets that need an X axis
  // (line/area/bar); pie reads raw rows and table reads columns. `needsX` and
  // `allowsBreakdown` come from the catalog, so this stays correct as widgets are
  // added without editing a per-type list here.
  const {
    chartData,
    keys,
    chartConfig: uiChartConfig,
  } = useMemo(() => {
    if (!needsX(config.type) || !config.x || !config.y) {
      return { chartData: [], keys: new Set<string>(), chartConfig: {} }
    }

    const xColumn = columns.find((col) => col.name === config.x)
    const yColumn = columns.find((col) => col.name === config.y)
    if (!xColumn || !yColumn) {
      return { chartData: [], keys: new Set<string>(), chartConfig: {} }
    }

    const breakdownColumn =
      allowsBreakdown(config.type) && config.breakdown
        ? columns.find((col) => col.name === config.breakdown)
        : undefined

    if (breakdownColumn) {
      return transformDataForBreakdown(data, config.x, config.y, config.breakdown!)
    }
    return transformDataForSimpleChart(data, config.x, [config.y])
  }, [config, data, columns])

  // Validate against the catalog's roles — one rule, not a hand-written gate.
  const missing = missingFields(config.type, { x: config.x, y: config.y })
  if (missing.length > 0) {
    return (
      <div className="flex items-center justify-center h-full w-full text-muted-foreground">
        <div className="text-center">
          <p className="text">Invalid chart configuration</p>
          {missing.map((m) => (
            <p key={m} className="text-sm mt-1">
              • {m} is required
            </p>
          ))}
        </div>
      </div>
    )
  }

  // Cartesian widgets with nothing to plot show an empty state; pie/table render
  // their own emptiness, so only gate when this widget builds series via X.
  if (needsX(config.type) && config.type !== ChartType.PieChart && keys.size === 0) {
    return (
      <div className="flex flex-1 h-full justify-center items-center bg-muted/30 rounded-lg">
        <span className="text-muted-foreground">No data during this period</span>
      </div>
    )
  }

  const ctx: WidgetRenderContext = {
    config,
    data,
    chartData,
    keys: Array.from(keys),
    uiChartConfig: uiChartConfig || generateChartConfig(Array.from(keys)),
    columns,
    hiddenColumns,
    onBarClick,
    onColumnConfigChange,
    syncId,
    drag,
    hasMore,
    isFetching,
    fetchNextPage,
  }

  // Dispatch through the renderMap — one entry per widget, no switch to drift.
  // `descriptor` falls back to a valid type for a stale stored value, so the
  // lookup is always defined.
  return RENDER_MAP[descriptor(config.type).type](ctx)
}
