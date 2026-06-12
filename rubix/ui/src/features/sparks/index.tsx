import { useMemo, useState } from 'react'
import { useAckSpark, useEquips, usePoints, useSparks } from '@/api/hooks'
import type { SparkSeverity, Uuid } from '@/api/types'
import { Main } from '@/components/layout/main'
import { PageHeader } from '@/components/layout/page-header'
import { Badge } from '@/components/ui/badge'
import { Card } from '@/components/ui/card'
import { Tabs, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { useActiveSite } from '@/hooks/use-active-site'
import { SparkDetail } from './components/spark-detail'
import { SparkListItem } from './components/spark-list-item'

type Filter = 'all' | SparkSeverity

export function Sparks() {
  const { site } = useActiveSite()
  const { data: sparks = [] } = useSparks(site?.id)
  const { data: points = [] } = usePoints({ siteId: site?.id })
  const { data: equips = [] } = useEquips(site?.id)
  const ack = useAckSpark(site?.id)
  const [filter, setFilter] = useState<Filter>('all')
  const [selectedId, setSelectedId] = useState<Uuid | undefined>()

  const sorted = useMemo(
    () => [...sparks].sort((a, b) => b.ts.localeCompare(a.ts)),
    [sparks]
  )
  const filtered = filter === 'all' ? sorted : sorted.filter((s) => s.severity === filter)
  const selected = filtered.find((s) => s.id === selectedId) ?? filtered[0]

  const counts = {
    all: sorted.length,
    fault: sorted.filter((s) => s.severity === 'fault').length,
    warning: sorted.filter((s) => s.severity === 'warning').length,
    info: sorted.filter((s) => s.severity === 'info').length,
  }
  const equipOf = (pointIds: Uuid[]) => {
    const eqId = points.find((p) => pointIds.includes(p.id))?.equip_id
    return equips.find((e) => e.id === eqId)?.display_name
  }

  return (
    <>
      <PageHeader title='Sparks' sub='Rule findings across your portfolio' />
      <Main fluid fixed className='flex min-h-0'>
        <div className='grid min-h-0 w-full flex-1 gap-4 lg:grid-cols-[330px_1fr]'>
          {/* master list */}
          <div className='flex min-h-0 flex-col gap-2.5'>
            <Tabs value={filter} onValueChange={(v) => setFilter(v as Filter)}>
              <TabsList className='h-8 w-full justify-start'>
                <FilterTab value='all' label='All' count={counts.all} />
                <FilterTab value='fault' label='Faults' count={counts.fault} />
                <FilterTab value='warning' label='Warnings' count={counts.warning} />
                <FilterTab value='info' label='Info' count={counts.info} />
              </TabsList>
            </Tabs>
            <Card className='scroll min-h-0 flex-1 gap-1 overflow-y-auto p-1.5'>
              {filtered.length === 0 ? (
                <p className='text-muted-foreground py-12 text-center text-sm'>No findings.</p>
              ) : (
                filtered.map((s) => (
                  <SparkListItem
                    key={s.id}
                    spark={s}
                    equipName={equipOf(s.point_ids)}
                    active={s.id === selected?.id}
                    onClick={() => setSelectedId(s.id)}
                  />
                ))
              )}
            </Card>
          </div>

          {/* detail */}
          <div className='scroll min-h-0 overflow-y-auto pe-1'>
            {selected ? (
              <SparkDetail
                spark={selected}
                site={site}
                points={points}
                equips={equips}
                onAck={() => ack.mutate(selected.id)}
                acking={ack.isPending}
              />
            ) : (
              <Card className='grid h-full place-items-center'>
                <p className='text-muted-foreground text-sm'>Select a finding.</p>
              </Card>
            )}
          </div>
        </div>
      </Main>
    </>
  )
}

function FilterTab({ value, label, count }: { value: Filter; label: string; count: number }) {
  return (
    <TabsTrigger value={value} className='gap-1.5 px-2.5 text-xs'>
      {label}
      <Badge variant='muted' className='h-4 px-1 text-[9.5px]'>
        {count}
      </Badge>
    </TabsTrigger>
  )
}
