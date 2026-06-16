/**
 * The themed recharts tooltip for the trend widgets (line/area/bar). recharts'
 * DEFAULT tooltip is a stark white box with a hard border that ignores the app
 * theme; this renders a rounded popover-coloured card with a colour swatch per
 * series instead, so the charts match the rest of the UI in light and dark mode.
 * The shared axis/grid constants live in chart-axes.ts (constants-only).
 */
import type { TooltipContentProps } from 'recharts'
import { formatTick } from './format-value'

type ChartTooltipProps = Partial<TooltipContentProps<number, string>> & {
  /** Formats the row label (a timestamp → local time). */
  labelFormat?: (label: string | number) => string
  /** Unit suffix appended to each value. */
  unit?: string
}

export function ChartTooltip({
  active,
  payload,
  label,
  labelFormat,
  unit,
}: ChartTooltipProps) {
  if (!active || !payload || payload.length === 0) return null
  return (
    <div className='bg-popover text-popover-foreground rounded-md border px-2.5 py-2 text-xs shadow-md'>
      <div className='text-muted-foreground mb-1 font-medium'>
        {labelFormat ? labelFormat(label as string | number) : label}
      </div>
      <div className='grid gap-1'>
        {payload.map((p) => (
          <div key={String(p.dataKey)} className='flex items-center gap-2'>
            <span
              className='size-2 rounded-[2px]'
              style={{ background: p.color }}
            />
            <span className='text-muted-foreground'>{p.name}</span>
            <span className='ml-auto font-medium tabular-nums'>
              {formatTick(Number(p.value))}
              {unit ? ` ${unit}` : ''}
            </span>
          </div>
        ))}
      </div>
    </div>
  )
}
