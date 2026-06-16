// The widget catalog (§8) — the load-bearing descriptor registry adopted from
// nexus's `catalog.ts`. One entry per chart type carries its picker label and
// field roles. This is the SINGLE source of truth the whole widget system reads:
//
//   - the type picker (which widgets exist),
//   - the config bar (which axis pickers to show — X, Y, Breakdown),
//   - the renderer (whether the config is complete enough to draw),
//   - the renderMap (which component renders it).
//
// So adding a widget is one catalog entry + one renderMap entry — never a fourth
// edit to a hand-written validation gate or a dispatch switch that drifts. This is
// the "pure option-builder + registry" discipline the scope calls out.

import { ChartType } from "./types";

/** The field roles a widget needs — what the config bar offers and the renderer
 *  validates. Derived predicates ([`needsX`] etc.) read these so no caller
 *  hard-codes a per-type rule. */
export interface FieldRoles {
  /** `required` → the x/category column is mandatory; `none` → no x axis (table). */
  x: "required" | "none";
  /** `required` → a y/value column is mandatory; `none` → no value column. */
  y: "required" | "none";
  /** `single` → one value series; `multi` → a Breakdown split into many series. */
  series: "single" | "multi";
}

/** A pure, library-agnostic widget descriptor. */
export interface WidgetDescriptor {
  type: ChartType;
  label: string;
  roles: FieldRoles;
}

/** Every chart type, keyed by `ChartType` so the compiler forces an entry for
 *  each. The easy tier (line/area/bar/pie) is recharts; table is DOM. Gauge and
 *  heatmap (the hard tier) stay on a deferred lazy ECharts island (§8) and are
 *  intentionally absent here. */
export const WIDGET_CATALOG: Record<ChartType, WidgetDescriptor> = {
  [ChartType.LineChart]: {
    type: ChartType.LineChart,
    label: "Line",
    roles: { x: "required", y: "required", series: "multi" },
  },
  [ChartType.AreaChart]: {
    type: ChartType.AreaChart,
    label: "Area",
    roles: { x: "required", y: "required", series: "multi" },
  },
  [ChartType.BarChart]: {
    type: ChartType.BarChart,
    label: "Bar",
    roles: { x: "required", y: "required", series: "multi" },
  },
  [ChartType.HorizontalBarChart]: {
    type: ChartType.HorizontalBarChart,
    label: "Horizontal bar",
    roles: { x: "required", y: "required", series: "single" },
  },
  [ChartType.PieChart]: {
    type: ChartType.PieChart,
    label: "Pie",
    roles: { x: "required", y: "required", series: "single" },
  },
  [ChartType.Table]: {
    type: ChartType.Table,
    label: "Table",
    roles: { x: "none", y: "none", series: "multi" },
  },
};

/** The catalog in declaration order — drives the type picker. */
export const WIDGETS: ReadonlyArray<WidgetDescriptor> = [
  WIDGET_CATALOG[ChartType.LineChart],
  WIDGET_CATALOG[ChartType.AreaChart],
  WIDGET_CATALOG[ChartType.BarChart],
  WIDGET_CATALOG[ChartType.HorizontalBarChart],
  WIDGET_CATALOG[ChartType.PieChart],
  WIDGET_CATALOG[ChartType.Table],
];

/** The descriptor for a type, defaulting to Line if a stale type is stored. */
export function descriptor(type: ChartType | undefined): WidgetDescriptor {
  return (type && WIDGET_CATALOG[type]) || WIDGET_CATALOG[ChartType.LineChart];
}

// --- Derived role predicates — the one place per-type field rules live. ---

/** Whether this widget needs an X/category column. */
export function needsX(type: ChartType | undefined): boolean {
  return descriptor(type).roles.x === "required";
}

/** Whether this widget needs a Y/value column. */
export function needsY(type: ChartType | undefined): boolean {
  return descriptor(type).roles.y === "required";
}

/** Whether this widget can split into multiple series via a Breakdown column. */
export function allowsBreakdown(type: ChartType | undefined): boolean {
  return descriptor(type).roles.series === "multi";
}

/** The missing required field names for `type`, given which columns are set. An
 *  empty array means the config is complete enough to render. */
export function missingFields(
  type: ChartType | undefined,
  has: { x?: string; y?: string },
): string[] {
  const missing: string[] = [];
  if (!type) missing.push("chart type");
  if (needsX(type) && !has.x) missing.push("X-axis column");
  if (needsY(type) && !has.y) missing.push("Y-axis column");
  return missing;
}
