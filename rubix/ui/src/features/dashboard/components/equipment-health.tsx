import { useNavigate } from '@tanstack/react-router'
import { CircuitBoard } from 'lucide-react'
import { useEquips } from '@/api/hooks'
import type { Equip, Uuid } from '@/api/types'
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card'
import { Skeleton } from '@/components/ui/skeleton'

/** Live equipment roster for the active site, linking into the points browser. */
export function EquipmentHealth({ siteId }: { siteId: Uuid | undefined }) {
  const navigate = useNavigate()
  const { data: equips = [], isLoading } = useEquips(siteId)

  return (
    <Card className='col-span-1 lg:col-span-2'>
      <CardHeader>
        <CardTitle>Equipment</CardTitle>
        <CardDescription>{equips.length} units on this site</CardDescription>
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
                onClick={() => navigate({ to: '/points', search: { equip: e.id } })}
              />
            ))}
          </div>
        )}
      </CardContent>
    </Card>
  )
}

function EquipTile({ equip, onClick }: { equip: Equip; onClick: () => void }) {
  return (
    <button
      onClick={onClick}
      className='bg-muted/40 hover:bg-accent hover:border-border-strong focus-visible:ring-ring flex items-center gap-3 rounded-lg border border-border px-3 py-2.5 text-left transition-colors focus-visible:ring-2 focus-visible:outline-none'
    >
      <span className='bg-card text-muted-foreground grid size-9 shrink-0 place-items-center rounded-lg border border-border'>
        <CircuitBoard className='size-[18px]' />
      </span>
      <div className='min-w-0 flex-1'>
        <div className='truncate text-[12.5px] font-semibold'>{equip.display_name}</div>
        <div className='text-muted-foreground truncate font-mono text-[11px]'>{equip.path}</div>
      </div>
    </button>
  )
}
