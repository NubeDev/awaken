import { useState } from 'react'
import { useSearch } from '@tanstack/react-router'
import { useEquips, usePoints, useSparks } from '@/api/hooks'
import type { Uuid } from '@/api/types'
import { Main } from '@/components/layout/main'
import { PageHeader } from '@/components/layout/page-header'
import { Card } from '@/components/ui/card'
import { useActiveSite } from '@/hooks/use-active-site'
import { EquipBrowser } from './components/equip-browser'
import { PointDetail } from './components/point-detail'
import { PointList } from './components/point-list'

export function Points() {
  const { site } = useActiveSite()
  // Read `?equip=` route-agnostically (the page mounts under the scoped tree).
  const { equip: equipParam } = useSearch({ strict: false }) as {
    equip?: string
  }
  const { data: equips = [] } = useEquips(site?.id)
  const { data: sitePoints = [] } = usePoints({ siteId: site?.id })
  const { data: sparks = [] } = useSparks(site?.id)

  // points implicated in an open finding (drives the red dots)
  const inFinding = new Set<Uuid>(
    sparks.filter((s) => !s.acknowledged).flatMap((s) => s.point_ids)
  )
  const pointEquip = new Map(sitePoints.map((p) => [p.id, p.equip_id]))
  const faultEquips = new Set(
    sparks
      .filter((s) => !s.acknowledged && s.severity === 'fault')
      .flatMap((s) => s.point_ids.map((pid) => pointEquip.get(pid)))
  )

  const [equipId, setEquipId] = useState<Uuid | undefined>(equipParam)
  // Adjust selection during render when the URL `?equip=` changes (e.g. a deep
  // link from another page) without an effect — the pattern React recommends
  // over syncing props into state in useEffect.
  const [lastParam, setLastParam] = useState(equipParam)
  if (equipParam !== lastParam) {
    setLastParam(equipParam)
    if (equipParam) setEquipId(equipParam)
  }

  // default to the showcase equip: one with an in-finding point, else one with points
  const defaultEquipId =
    sitePoints.find((p) => inFinding.has(p.id))?.equip_id ?? sitePoints[0]?.equip_id
  const activeEquip =
    equips.find((e) => e.id === (equipId ?? defaultEquipId)) ?? equips[0]

  const { data: points = [], isLoading } = usePoints({ equipId: activeEquip?.id })
  const [pointId, setPointId] = useState<Uuid | undefined>()
  const activePoint =
    points.find((p) => p.id === pointId) ??
    points.find((p) => inFinding.has(p.id)) ??
    points[0]

  return (
    <>
      <PageHeader
        title='Points & Equipment'
        sub={site ? `${site.display_name} · ${sitePoints.length.toLocaleString()} points` : '…'}
      />
      <Main fluid fixed className='flex min-h-0'>
        <div className='grid min-h-0 w-full flex-1 gap-3 lg:grid-cols-[230px_320px_1fr]'>
          <Card className='scroll overflow-y-auto p-2.5'>
            <EquipBrowser
              site={site}
              equips={equips}
              faultEquips={faultEquips}
              activeId={activeEquip?.id}
              onSelect={(id) => {
                setEquipId(id)
                setPointId(undefined)
              }}
            />
          </Card>

          <Card className='scroll gap-0 overflow-y-auto py-3'>
            <PointList
              site={site}
              equip={activeEquip}
              points={points}
              loading={isLoading}
              inFinding={inFinding}
              activeId={activePoint?.id}
              onSelect={setPointId}
            />
          </Card>

          <div className='scroll min-h-0 overflow-y-auto pe-1'>
            {activePoint ? (
              <PointDetail
                key={activePoint.id}
                point={activePoint}
                site={site}
                equip={activeEquip}
                inFinding={inFinding.has(activePoint.id)}
              />
            ) : (
              <Card className='grid h-full place-items-center'>
                <p className='text-muted-foreground text-sm'>Select a point to inspect.</p>
              </Card>
            )}
          </div>
        </div>
      </Main>
    </>
  )
}
