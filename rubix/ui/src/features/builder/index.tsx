import { useState } from 'react'
import { useEquips, usePoints, useWidgets } from '@/api/hooks'
import { Card } from '@/components/ui/card'
import { Main } from '@/components/layout/main'
import { PageHeader } from '@/components/layout/page-header'
import { useActiveSite } from '@/hooks/use-active-site'
import { WidgetBinder } from './components/widget-binder'
import { WidgetCanvas } from './components/widget-canvas'
import { WidgetPalette } from './components/widget-palette'
import type { PaletteEntry } from './lib/palette'

type Bindable = Extract<PaletteEntry, { available: true }>

/**
 * Dashboard Builder: pin tiles to live points or boards through the real
 * `/api/v1/widgets` store, arrange them on a grid, and render them live. Pins
 * persist as server rows so they survive reload.
 */
export function Builder() {
  const { site } = useActiveSite()
  const { data: widgets = [] } = useWidgets(site?.id)
  const { data: equips = [] } = useEquips(site?.id)
  const { data: points = [] } = usePoints({ siteId: site?.id })
  const [picked, setPicked] = useState<Bindable | null>(null)

  return (
    <>
      <PageHeader title='Dashboard Builder' sub='Compose and bind widgets to live data' />
      <Main fluid fixed className='flex min-h-0'>
        <div className='grid min-h-0 w-full flex-1 gap-3 lg:grid-cols-[230px_1fr]'>
          <Card className='scroll overflow-y-auto p-2.5'>
            {site ? (
              <WidgetPalette onPick={setPicked} />
            ) : (
              <p className='text-muted-foreground text-[12px]'>Loading site…</p>
            )}
          </Card>

          <div className='scroll min-h-0 overflow-y-auto pe-1'>
            {site ? (
              <WidgetCanvas site={site} widgets={widgets} equips={equips} points={points} />
            ) : (
              <Card className='grid h-full place-items-center'>
                <p className='text-muted-foreground text-sm'>Loading site…</p>
              </Card>
            )}
          </div>
        </div>
      </Main>

      {site ? (
        <WidgetBinder site={site} entry={picked} onClose={() => setPicked(null)} />
      ) : null}
    </>
  )
}
