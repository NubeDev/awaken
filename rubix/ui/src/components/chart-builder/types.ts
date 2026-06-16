export enum ChartType {
  "LineChart" = "line",
  "AreaChart" = "area",
  "BarChart" = "bar",
  "HorizontalBarChart" = "horizontalBar",
  "PieChart" = "pie",
  "Table" = "table",
}

export type DisplayMode = "total" | "average" | "none";

import type { FieldConfig } from "./field-config";
import type { PhysicalQuantity } from "./units";
import type { Transform } from "./transforms";

interface BaseChartConfig {
  x?: string;
  y?: string;
  breakdown?: string;
  /** @deprecated Use displayMode instead. Kept for backward compatibility. */
  total?: boolean;
  displayMode?: DisplayMode;
  /** Grafana-style per-field display config + overrides (§7). Additive and
   *  optional; absence renders exactly as the MVP config did. */
  fieldConfig?: FieldConfig;
  /** Per-column physical quantity, authored here so the query API converts each
   *  column to the caller's unit system (§2/§7). Keyed by result-column name;
   *  threaded into the batch request's `quantities` map. The cache holds raw
   *  canonical values — conversion is a post-read, per-caller layer. */
  quantities?: Record<string, PhysicalQuantity>;
  /** Portable post-query transform pipeline (§1). The aggregate ops
   *  (filter/groupBy/reduce) are sent with the request and run server-side; the
   *  cosmetic ops (rename/calculated/organize) run client-side after the rows
   *  return. Absence → rows pass through. */
  transforms?: Transform[];
  /** A saved-query id this chart references instead of embedding `sql` (§4b).
   *  Resolved to SQL server-side on the caller's scope. */
  query_id?: string;
}

export interface AxisChartConfig extends BaseChartConfig {
  type?:
    | ChartType.LineChart
    | ChartType.AreaChart
    | ChartType.BarChart
    | ChartType.HorizontalBarChart
    | ChartType.PieChart;
}

export interface TableColumnConfig {
  columnOrder?: string[];
  columnSizing?: Record<string, number>;
  columnVisibility?: Record<string, boolean>;
}

export interface TableChartConfig extends BaseChartConfig {
  type: ChartType.Table;
  tableColumnConfig?: TableColumnConfig;
}

export type ChartConfig = AxisChartConfig | TableChartConfig;

export const isTableConfig = (config: ChartConfig): config is TableChartConfig => config.type === ChartType.Table;

/** Resolve displayMode from config, with backward compatibility for `total: true`. */
export const resolveDisplayMode = (config: ChartConfig): DisplayMode => {
  if (config.displayMode) return config.displayMode;
  if (config.total) return "total";
  return "none";
};
