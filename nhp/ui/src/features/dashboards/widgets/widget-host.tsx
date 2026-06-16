/**
 * Widget registry / render dispatch (DASHBOARDS-SCOPE §7: "pure option-builder +
 * registry; renderWidget dispatches"). A trend widget (line/bar/area) carries its
 * own `type`; this maps it to the recharts renderer. The panel chrome (title +
 * card) lives here so every widget gets the same frame. PURE — no fetching.
 *
 * Only line/bar/area route through here; stat/table/status/alarm have their own
 * card chrome and are placed directly by the page (they are DOM tiles, not the
 * cartesian-chart family). ECharts is NOT imported — no seeded register needs a
 * gauge/heatmap, so the POC ships recharts only and the lazy ECharts island is
 * omitted (DASHBOARDS-SCOPE §8: don't pay for it when nothing uses it).
 */
import { Card } from '@/components/ui/card'
import { AreaWidget } from './area'
import { BarWidget } from './bar'
import { LineWidget } from './line'
import type { TrendWidget } from './types'

export function TrendPanel({ widget }: { widget: TrendWidget }) {
  return (
    <Card className='p-4'>
      <div className='mb-2 text-sm font-medium'>{widget.title}</div>
      {widget.type === 'bar' ? (
        <BarWidget widget={widget} />
      ) : widget.type === 'area' ? (
        <AreaWidget widget={widget} />
      ) : (
        <LineWidget widget={widget} />
      )}
    </Card>
  )
}
