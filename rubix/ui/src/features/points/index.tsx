import { useEffect, useState } from 'react'
import { getRouteApi } from '@tanstack/react-router'
import { CircuitBoard } from 'lucide-react'
import { useEquips, usePoints } from '@/api/hooks'
import type { Uuid } from '@/api/types'
import { ConfigDrawer } from '@/components/config-drawer'
import { Header } from '@/components/layout/header'
import { Main } from '@/components/layout/main'
import { ProfileDropdown } from '@/components/profile-dropdown'
import { Search } from '@/components/search'
import { ThemeSwitch } from '@/components/theme-switch'
import { Card, CardContent } from '@/components/ui/card'
import { Skeleton } from '@/components/ui/skeleton'
import { useActiveSite } from '@/hooks/use-active-site'
import { cn } from '@/lib/utils'
import { formatValue } from '@/lib/format'
import { PointDetail } from './components/point-detail'

const route = getRouteApi('/_authenticated/points/')

export function Points() {
  const { site } = useActiveSite()
  const { equip: equipParam } = route.useSearch()
  const { data: equips = [] } = useEquips(site?.id)
  // Site-wide points let us default to an equip that actually has points,
  // so the detail panel (priority-array showcase) isn't empty on first load.
  const { data: sitePoints = [] } = usePoints({ siteId: site?.id })

  const [equipId, setEquipId] = useState<Uuid | undefined>(equipParam)
  const defaultEquipId = sitePoints[0]?.equip_id ?? equips[0]?.id
  const activeEquip =
    equips.find((e) => e.id === (equipId ?? defaultEquipId)) ?? equips[0]

  useEffect(() => {
    if (equipParam) setEquipId(equipParam)
  }, [equipParam])

  const { data: points = [], isLoading } = usePoints({ equipId: activeEquip?.id })
  const [pointId, setPointId] = useState<Uuid | undefined>()
  const activePoint = points.find((p) => p.id === pointId) ?? points[0]

  return (
    <>
      <Header>
        <Search />
        <div className='ms-auto flex items-center gap-2'>
          <ThemeSwitch />
          <ConfigDrawer />
          <ProfileDropdown />
        </div>
      </Header>
      <Main>
        <div className='mb-4'>
          <h1 className='text-2xl font-bold tracking-tight'>Points &amp; Equipment</h1>
          <p className='text-muted-foreground text-sm'>
            {site ? `${site.display_name} · ${equips.length} equips` : 'Loading…'}
          </p>
        </div>

        <div className='grid gap-4 lg:grid-cols-[220px_1fr_340px]'>
          {/* Equipment list */}
          <Card className='h-fit'>
            <CardContent className='p-2'>
              <div className='space-y-0.5'>
                {equips.map((e) => (
                  <button
                    key={e.id}
                    onClick={() => {
                      setEquipId(e.id)
                      setPointId(undefined)
                    }}
                    className={cn(
                      'hover:bg-accent flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-[12.5px]',
                      e.id === activeEquip?.id && 'bg-accent font-medium'
                    )}
                  >
                    <CircuitBoard className='text-muted-foreground size-4 shrink-0' />
                    <span className='truncate'>{e.display_name}</span>
                  </button>
                ))}
              </div>
            </CardContent>
          </Card>

          {/* Point list */}
          <Card>
            <CardContent className='p-2'>
              {isLoading ? (
                <div className='space-y-1.5 p-1'>
                  {Array.from({ length: 8 }).map((_, i) => (
                    <Skeleton key={i} className='h-9 rounded-md' />
                  ))}
                </div>
              ) : points.length === 0 ? (
                <p className='text-muted-foreground py-10 text-center text-sm'>No points.</p>
              ) : (
                <table className='w-full text-[12.5px]'>
                  <tbody>
                    {points.map((p) => (
                      <tr
                        key={p.id}
                        onClick={() => setPointId(p.id)}
                        className={cn(
                          'hover:bg-accent cursor-pointer',
                          p.id === activePoint?.id && 'bg-accent'
                        )}
                      >
                        <td className='rounded-s-md py-1.5 ps-2'>{p.display_name}</td>
                        <td className='text-muted-foreground py-1.5 font-mono text-[11px]'>
                          {p.kind}
                        </td>
                        <td className='tabular rounded-e-md py-1.5 pe-2 text-right font-medium'>
                          {formatValue(p.cur_value, p.unit)}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              )}
            </CardContent>
          </Card>

          {/* Detail */}
          <Card className='h-fit'>
            <CardContent className='p-4'>
              {activePoint ? (
                <PointDetail point={activePoint} />
              ) : (
                <p className='text-muted-foreground py-10 text-center text-sm'>
                  Select a point to inspect.
                </p>
              )}
            </CardContent>
          </Card>
        </div>
      </Main>
    </>
  )
}
