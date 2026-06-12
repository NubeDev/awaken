import { usePoints } from '@/api/hooks'
import type { Uuid } from '@/api/types'
import { Donut, type DonutSlice } from '@/components/charts/donut'
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card'

/** Composition of the site's points by kind — a real breakdown, donut + legend. */
export function PointsBreakdown({ siteId }: { siteId: Uuid | undefined }) {
  const { data: points = [] } = usePoints({ siteId })
  const counts = {
    sensor: points.filter((p) => p.kind === 'sensor').length,
    cmd: points.filter((p) => p.kind === 'cmd').length,
    sp: points.filter((p) => p.kind === 'sp').length,
  }
  const slices: DonutSlice[] = [
    { label: 'Sensors', value: counts.sensor, color: 'var(--chart-1)' },
    { label: 'Commands', value: counts.cmd, color: 'var(--chart-2)' },
    { label: 'Setpoints', value: counts.sp, color: 'var(--chart-4)' },
  ].filter((s) => s.value > 0)

  return (
    <Card>
      <CardHeader>
        <CardTitle>Point Mix</CardTitle>
        <CardDescription>By kind · live</CardDescription>
      </CardHeader>
      <CardContent className='flex items-center gap-5'>
        {slices.length === 0 ? (
          <p className='text-muted-foreground py-8 text-center text-sm'>No points.</p>
        ) : (
          <>
            <Donut data={slices} total={String(points.length)} totalLabel='points' />
            <div className='flex flex-1 flex-col gap-2.5'>
              {slices.map((s) => (
                <div key={s.label} className='flex items-center gap-2.5 text-[12.5px]'>
                  <span className='size-2.5 rounded-sm' style={{ background: s.color }} />
                  <span className='text-muted-foreground flex-1'>{s.label}</span>
                  <span className='tabular font-semibold'>{s.value}</span>
                </div>
              ))}
            </div>
          </>
        )}
      </CardContent>
    </Card>
  )
}
