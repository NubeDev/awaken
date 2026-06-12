import { useNavigate } from '@tanstack/react-router'
import { ArrowRight } from 'lucide-react'
import { useEquips, usePoints, useSparks } from '@/api/hooks'
import type { Equip, Uuid } from '@/api/types'
import { Button } from '@/components/ui/button'
import {
  Card,
  CardAction,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card'
import { Skeleton } from '@/components/ui/skeleton'
import { cn } from '@/lib/utils'
import { equipKindIcon } from '@/lib/equip-icon'

/** Live equipment roster: per-kind icons, fault status from open sparks. */
export function EquipmentHealth({ siteId }: { siteId: Uuid | undefined }) {
  const navigate = useNavigate()
  const { data: equips = [], isLoading } = useEquips(siteId)
  const { data: points = [] } = usePoints({ siteId })
  const { data: sparks = [] } = useSparks(siteId)

  // equips with an unacked spark on one of their points are in fault
  const pointEquip = new Map(points.map((p) => [p.id, p.equip_id]))
  const faultEquips = new Set(
    sparks
      .filter((s) => !s.acknowledged && s.severity === 'fault')
      .flatMap((s) => s.point_ids.map((pid) => pointEquip.get(pid)))
      .filter(Boolean)
  )
  const pointCount = new Map<string, number>()
  for (const p of points) pointCount.set(p.equip_id, (pointCount.get(p.equip_id) ?? 0) + 1)

  return (
    <Card className='col-span-1 lg:col-span-2'>
      <CardHeader>
        <CardTitle>Equipment Health</CardTitle>
        <CardDescription>
          {equips.length - faultEquips.size}/{equips.length} nominal
        </CardDescription>
        <CardAction>
          <Button variant='outline' size='sm' onClick={() => navigate({ to: '/points' })}>
            Open browser <ArrowRight className='size-3.5' />
          </Button>
        </CardAction>
      </CardHeader>
      <CardContent>
        {isLoading ? (
          <div className='grid gap-2.5 sm:grid-cols-2 xl:grid-cols-3'>
            {Array.from({ length: 6 }).map((_, i) => (
              <Skeleton key={i} className='h-[58px] rounded-lg' />
            ))}
          </div>
        ) : equips.length === 0 ? (
          <p className='text-muted-foreground py-8 text-center text-sm'>
            No equipment on this site yet.
          </p>
        ) : (
          <div className='grid gap-2.5 sm:grid-cols-2 xl:grid-cols-3'>
            {equips.map((e) => (
              <EquipTile
                key={e.id}
                equip={e}
                fault={faultEquips.has(e.id)}
                points={pointCount.get(e.id) ?? 0}
                onClick={() => navigate({ to: '/points', search: { equip: e.id } })}
              />
            ))}
          </div>
        )}
      </CardContent>
    </Card>
  )
}

function EquipTile({
  equip,
  fault,
  points,
  onClick,
}: {
  equip: Equip
  fault: boolean
  points: number
  onClick: () => void
}) {
  const Icon = equipKindIcon(equip.tags)
  return (
    <button
      onClick={onClick}
      className='group bg-muted/40 hover:bg-accent hover:border-border-strong focus-visible:ring-ring flex items-center gap-3 rounded-lg border border-border px-3 py-2.5 text-left transition-all duration-150 hover:-translate-y-px hover:shadow-md focus-visible:ring-2 focus-visible:outline-none'
    >
      <span
        className={cn(
          'bg-card grid size-9 shrink-0 place-items-center rounded-lg border border-border transition-colors',
          fault ? 'text-sev-fault' : 'text-muted-foreground group-hover:text-foreground'
        )}
      >
        <Icon className='size-[18px]' />
      </span>
      <div className='min-w-0 flex-1'>
        <div className='truncate text-[12.5px] font-semibold'>{equip.display_name}</div>
        <div className='text-muted-foreground flex items-center gap-1.5 text-[11px]'>
          <span
            className={cn('size-1.5 rounded-full', fault ? 'bg-sev-fault' : 'bg-positive')}
          />
          {fault ? 'Fault active' : 'Nominal'}
          {points > 0 ? ` · ${points} pts` : ''}
        </div>
      </div>
    </button>
  )
}
