import { usePoints } from '@/api/hooks'
import type { Uuid } from '@/api/types'
import { hasTag } from '@/api/tags'
import { Donut, type DonutSlice } from '@/components/charts/donut'
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card'

const COLORS = ['var(--chart-1)', 'var(--chart-2)', 'var(--chart-3)', 'var(--chart-4)', 'var(--chart-5)']

/** Per-system electrical load split, read from the site's submeter points. */
export function LoadBreakdown({ siteId }: { siteId: Uuid | undefined }) {
  const { data: points = [] } = usePoints({ siteId })
  const submeters = points.filter(
    (p) => hasTag(p.tags, 'submeter') && typeof p.cur_value === 'number'
  )
  const slices: DonutSlice[] = submeters.map((p, i) => ({
    label: p.display_name,
    value: p.cur_value as number,
    color: COLORS[i % COLORS.length]!,
  }))
  const total = Math.round(slices.reduce((a, s) => a + s.value, 0))

  return (
    <Card>
      <CardHeader>
        <CardTitle>Load Breakdown</CardTitle>
        <CardDescription>By system · live</CardDescription>
      </CardHeader>
      <CardContent className='flex items-center gap-5'>
        {slices.length === 0 ? (
          <p className='text-muted-foreground w-full py-10 text-center text-sm'>
            No submeters on this site.
          </p>
        ) : (
          <>
            <Donut data={slices} total={String(total)} totalLabel='kW total' size={150} />
            <div className='flex flex-1 flex-col gap-2.5'>
              {slices.map((s) => (
                <div key={s.label} className='flex items-center gap-2.5 text-[12.5px]'>
                  <span className='size-2.5 shrink-0 rounded-sm' style={{ background: s.color }} />
                  <span className='text-muted-foreground flex-1 truncate'>{s.label}</span>
                  <span className='tabular font-semibold'>
                    {Math.round(s.value)}
                    <span className='text-muted-foreground font-normal'> kW</span>
                  </span>
                </div>
              ))}
            </div>
          </>
        )}
      </CardContent>
    </Card>
  )
}
