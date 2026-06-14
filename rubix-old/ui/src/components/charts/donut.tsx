import { Cell, Pie, PieChart, ResponsiveContainer } from 'recharts'

export type DonutSlice = { label: string; value: number; color: string }

type DonutProps = {
  data: DonutSlice[]
  total?: string
  totalLabel?: string
  size?: number
}

/** Donut with a centred total, used for categorical breakdowns. */
export function Donut({ data, total, totalLabel, size = 140 }: DonutProps) {
  return (
    <div className='relative' style={{ width: size, height: size }}>
      <ResponsiveContainer width='100%' height='100%'>
        <PieChart>
          <Pie
            data={data}
            dataKey='value'
            nameKey='label'
            innerRadius={size * 0.32}
            outerRadius={size * 0.48}
            paddingAngle={2}
            stroke='none'
            isAnimationActive={false}
          >
            {data.map((d) => (
              <Cell key={d.label} fill={d.color} />
            ))}
          </Pie>
        </PieChart>
      </ResponsiveContainer>
      {total ? (
        <div className='pointer-events-none absolute inset-0 flex flex-col items-center justify-center'>
          <span className='tabular text-xl font-semibold tracking-tight'>{total}</span>
          {totalLabel ? (
            <span className='text-muted-foreground text-[11px]'>{totalLabel}</span>
          ) : null}
        </div>
      ) : null}
    </div>
  )
}
