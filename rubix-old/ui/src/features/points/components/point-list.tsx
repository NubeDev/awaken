import { useState } from 'react'
import { Plus } from 'lucide-react'
import { tagNames } from '@/api/tags'
import type { Equip, Point, Site, Uuid } from '@/api/types'
import { ageShort, formatValue } from '@/lib/format'
import { cn } from '@/lib/utils'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Skeleton } from '@/components/ui/skeleton'
import { PointFormDialog } from './point-form-dialog'
import { PointKindIcon } from './point-kind-icon'

type PointListProps = {
  site: Site | undefined
  equip: Equip | undefined
  points: Point[]
  loading: boolean
  inFinding: Set<Uuid>
  activeId: Uuid | undefined
  onSelect: (id: Uuid) => void
}

/** Middle pane: the selected equip's points with live values and freshness. */
export function PointList({
  site,
  equip,
  points,
  loading,
  inFinding,
  activeId,
  onSelect,
}: PointListProps) {
  const [createOpen, setCreateOpen] = useState(false)
  return (
    <div className='flex h-full flex-col'>
      <div className='border-b border-border px-3 pb-3'>
        <div className='flex items-center justify-between gap-2'>
          <div className='min-w-0'>
            <div className='truncate text-[13.5px] font-semibold'>
              {equip?.display_name ?? '…'}
            </div>
            <div className='truncate font-mono text-[11px] text-muted-foreground'>
              {site && equip ? `${site.org}/${site.slug}/${equip.path}` : ''}
            </div>
          </div>
          <div className='flex shrink-0 items-center gap-1'>
            <Badge variant='muted'>{points.length} pts</Badge>
            {equip ? (
              <Button
                size='icon'
                variant='ghost'
                className='size-6'
                title='Add point'
                onClick={() => setCreateOpen(true)}
              >
                <Plus className='size-3.5' />
              </Button>
            ) : null}
          </div>
        </div>
        {equip && tagNames(equip.tags).length > 0 ? (
          <div className='mt-2 flex flex-wrap gap-1'>
            {tagNames(equip.tags).map((t) => (
              <Badge
                key={t}
                variant='outline'
                className='h-[18px] px-1.5 font-mono text-[10px]'
              >
                #{t}
              </Badge>
            ))}
          </div>
        ) : null}
      </div>

      <div className='flex-1 space-y-0.5 overflow-y-auto p-2'>
        {loading ? (
          Array.from({ length: 6 }).map((_, i) => (
            <Skeleton key={i} className='h-12 rounded-lg' />
          ))
        ) : points.length === 0 ? (
          <p className='py-10 text-center text-sm text-muted-foreground'>
            No points on this equip.
          </p>
        ) : (
          points.map((p) => {
            const active = p.id === activeId
            return (
              <button
                key={p.id}
                onClick={() => onSelect(p.id)}
                className={cn(
                  'flex w-full items-center gap-3 rounded-lg px-2.5 py-2 text-left transition-colors hover:bg-accent',
                  active && 'bg-accent'
                )}
              >
                <PointKindIcon kind={p.kind} inFinding={inFinding.has(p.id)} />
                <div className='min-w-0 flex-1'>
                  <div className='truncate text-[12.5px] font-medium'>
                    {p.display_name}
                  </div>
                  <div className='truncate font-mono text-[10.5px] text-muted-foreground'>
                    {p.slug}
                  </div>
                </div>
                <div className='shrink-0 text-end'>
                  <div className='tabular text-[13px] font-semibold'>
                    {formatValue(p.cur_value, p.unit)}
                  </div>
                  <div className='text-[10.5px] text-muted-foreground'>
                    {ageShort(p.cur_ts)}
                  </div>
                </div>
              </button>
            )
          })
        )}
      </div>

      {equip ? (
        <PointFormDialog
          mode='create'
          equipId={equip.id}
          open={createOpen}
          onOpenChange={setCreateOpen}
        />
      ) : null}
    </div>
  )
}
