import { useMemo } from "react";
import { Cell, Pie, PieChart as RechartsPieChart } from "recharts";

import { type ChartConfig, ChartContainer, ChartTooltip, ChartTooltipContent } from "@/components/ui/chart";
import { type FieldConfig } from "@/components/chart-builder/field-config";

import { fieldValueFormatter, pointRampColor } from "./field-format";

// Easy-tier nexus widget ported to recharts (§8). One slice per row: `x` is the
// slice label (category), `y` the slice value. The §7 FieldConfig drives the
// value formatting in the tooltip and, when a threshold ramp is set, the per-slice
// colour (else the chart palette cycles). Pure function of {config, rows}.
interface PieChartProps {
  data: Record<string, any>[];
  x: string;
  y: string;
  fieldConfig?: FieldConfig;
}

const PALETTE = [
  "hsl(var(--chart-1))",
  "hsl(var(--chart-2))",
  "hsl(var(--chart-3))",
  "hsl(var(--chart-4))",
  "hsl(var(--chart-5))",
];

const PieChartWidget = ({ data, x, y, fieldConfig }: PieChartProps) => {
  const valueFormatter = useMemo(() => fieldValueFormatter(y, fieldConfig), [y, fieldConfig]);

  // One slice per row; coerce the value column to a number, drop non-finite.
  const slices = useMemo(
    () =>
      data
        .map((row) => ({ name: String(row[x] ?? ""), value: Number(row[y]) }))
        .filter((s) => Number.isFinite(s.value)),
    [data, x, y]
  );

  // A chartConfig keyed by slice name so the shared tooltip/legend resolve labels.
  const chartConfig = useMemo<ChartConfig>(
    () =>
      Object.fromEntries(
        slices.map((s, i) => [s.name, { label: s.name, color: PALETTE[i % PALETTE.length] }])
      ),
    [slices]
  );

  if (slices.length === 0) {
    return (
      <div className="flex h-full items-center justify-center text-muted-foreground">
        <span>No data during this period</span>
      </div>
    );
  }

  return (
    <ChartContainer config={chartConfig} className="aspect-auto h-full w-full">
      <RechartsPieChart>
        <ChartTooltip
          content={<ChartTooltipContent nameKey="name" formatter={(value) => valueFormatter(value)} />}
        />
        <Pie data={slices} dataKey="value" nameKey="name" innerRadius="40%" outerRadius="75%" paddingAngle={1}>
          {slices.map((s, i) => {
            // A threshold ramp (if authored) colours the slice by its value; else
            // the palette cycles.
            const ramp = pointRampColor(y, s.value, fieldConfig);
            return <Cell key={s.name} fill={ramp ?? PALETTE[i % PALETTE.length]} />;
          })}
        </Pie>
      </RechartsPieChart>
    </ChartContainer>
  );
};

export default PieChartWidget;
