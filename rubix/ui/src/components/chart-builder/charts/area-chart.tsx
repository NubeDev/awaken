import React, { useMemo } from "react";
import {
  Area,
  AreaChart as RechartsAreaChart,
  CartesianGrid,
  ReferenceArea,
  XAxis,
  YAxis,
} from "recharts";

import { type ChartConfig, ChartContainer, ChartTooltip, ChartTooltipContent } from "@/components/ui/chart";
import { type DisplayMode } from "@/components/chart-builder/types";
import { type FieldConfig } from "@/components/chart-builder/field-config";

import { usePreferences } from "@/context/PreferencesContext";

import { type ChartDragHandlers } from "./line-chart";
import { fieldValueFormatter, seriesColor } from "./field-format";
import { formatMetricValue } from "./format-value";
import { calculateDisplayValue, createAxisFormatter } from "./utils";

// Easy-tier nexus widget ported to recharts (§8). A stacked-or-overlaid area over
// the same {x, y, breakdown} series the line/bar charts use, with the §7
// FieldConfig driving the value-axis units/decimals and per-series colour. Each
// series gets a soft gradient fill, the recharts area idiom.
interface AreaChartProps {
  data: Record<string, any>[];
  x: string;
  y: string;
  breakdown?: string;
  keys: string[];
  chartConfig: ChartConfig;
  displayMode?: DisplayMode;
  metricColumn?: string;
  fieldConfig?: FieldConfig;
  syncId?: string;
  drag?: ChartDragHandlers;
}

const AreaChart = ({
  data,
  x,
  keys,
  chartConfig,
  displayMode = "none",
  metricColumn,
  fieldConfig,
  syncId,
  drag,
}: AreaChartProps) => {
  const prefs = usePreferences();
  const xAxisFormatter = useMemo(() => createAxisFormatter(data, x, prefs), [data, x, prefs]);
  // The value axis honours the §7 FieldConfig (unit/decimals/mappings).
  const yAxisFormatter = useMemo(
    () => fieldValueFormatter(keys[0] || "", fieldConfig),
    [keys, fieldConfig]
  );

  const { displayValue, totalMax } = useMemo(
    () => calculateDisplayValue(data, keys, displayMode),
    [data, keys, displayMode]
  );

  return (
    <div className="flex flex-col overflow-hidden h-full">
      {displayValue !== null && (
        <span className="font-medium text-2xl mb-2 truncate min-h-fit">
          {formatMetricValue(displayValue, metricColumn)}
        </span>
      )}
      <ChartContainer config={chartConfig} className="aspect-auto flex-1 min-h-0 w-full">
        <RechartsAreaChart
          data={data}
          syncId={syncId}
          onMouseDown={drag?.onMouseDown}
          onMouseMove={drag?.onMouseMove}
          onMouseUp={drag?.onMouseUp}
          style={drag ? { userSelect: "none", cursor: "crosshair" } : undefined}
        >
          <defs>
            {keys.map((key) => {
              const color = seriesColor(key, chartConfig[key]?.color ?? "hsl(var(--chart-1))", fieldConfig);
              return (
                <linearGradient key={key} id={`fill-${key}`} x1="0" y1="0" x2="0" y2="1">
                  <stop offset="5%" stopColor={color} stopOpacity={0.4} />
                  <stop offset="95%" stopColor={color} stopOpacity={0.05} />
                </linearGradient>
              );
            })}
          </defs>
          <CartesianGrid vertical={false} />
          <XAxis
            type="category"
            tickLine={false}
            axisLine={false}
            tickMargin={8}
            dataKey={x}
            style={{ fill: "hsl(var(--muted-foreground))" }}
            tickFormatter={xAxisFormatter}
          />
          <YAxis
            tickLine={false}
            axisLine={false}
            tickCount={5}
            domain={["auto", totalMax]}
            width="auto"
            style={{ fill: "hsl(var(--muted-foreground))" }}
            tickFormatter={yAxisFormatter}
          />
          <ChartTooltip
            content={<ChartTooltipContent labelKey={x} labelFormatter={(_, p) => xAxisFormatter(p[0].payload[x])} />}
          />
          {keys.map((key) => {
            const color = seriesColor(key, chartConfig[key]?.color ?? "hsl(var(--chart-1))", fieldConfig);
            return (
              <Area
                key={key}
                dataKey={key}
                type="monotone"
                stroke={color}
                fill={`url(#fill-${key})`}
                stackId={keys.length > 1 ? "a" : undefined}
              />
            );
          })}
          {drag?.refArea.left && drag.refArea.right && (
            <ReferenceArea
              x1={drag.refArea.left}
              x2={drag.refArea.right}
              stroke="hsl(var(--primary))"
              strokeOpacity={0.5}
              fill="hsl(var(--primary))"
              fillOpacity={0.3}
            />
          )}
        </RechartsAreaChart>
      </ChartContainer>
    </div>
  );
};

export default AreaChart;
