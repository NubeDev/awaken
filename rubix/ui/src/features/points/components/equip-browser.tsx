import { useState } from 'react'
import { Building2, Search } from 'lucide-react'
import type { Equip, Site, Uuid } from '@/api/types'
import { tagNames } from '@/api/tags'
import { Input } from '@/components/ui/input'
import { cn } from '@/lib/utils'
import { EquipKindIcon } from '@/lib/equip-icon'

type EquipBrowserProps = {
  site: Site | undefined
  equips: Equip[]
  faultEquips: Set<Uuid | undefined>
  activeId: Uuid | undefined
  onSelect: (id: Uuid) => void
}

/** Left pane: filterable equipment tree for the active site. */
export function EquipBrowser({ site, equips, faultEquips, activeId, onSelect }: EquipBrowserProps) {
  const [filter, setFilter] = useState('')
  const q = filter.trim().toLowerCase()
  const visible = q
    ? equips.filter(
        (e) =>
          e.display_name.toLowerCase().includes(q) ||
          tagNames(e.tags).some((t) => `#${t}`.includes(q) || t.includes(q.replace(/^#/, '')))
      )
    : equips

  return (
    <div className='flex h-full flex-col gap-2'>
      <div className='relative'>
        <Search className='text-muted-foreground absolute start-2.5 top-1/2 size-3.5 -translate-y-1/2' />
        <Input
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
          placeholder='Filter equipment or #tag'
          className='h-8 ps-8 text-[12.5px]'
        />
      </div>
      <div className='text-muted-foreground flex items-center gap-2 px-1.5 pt-1 text-[11.5px] font-medium'>
        <Building2 className='size-3.5' />
        {site?.display_name ?? '…'}
      </div>
      <div className='flex flex-col gap-0.5'>
        {visible.map((e) => {
          const fault = faultEquips.has(e.id)
          const active = e.id === activeId
          return (
            <button
              key={e.id}
              onClick={() => onSelect(e.id)}
              className={cn(
                'hover:bg-accent flex w-full items-center gap-2.5 rounded-md px-2 py-[7px] text-left text-[12.5px] transition-colors',
                active && 'bg-accent font-medium'
              )}
            >
              <EquipKindIcon
                tags={e.tags}
                className={cn('size-4 shrink-0', active ? 'text-primary' : 'text-muted-foreground')}
              />
              <span className='flex-1 truncate'>{e.display_name}</span>
              {fault ? <span className='bg-sev-fault size-1.5 shrink-0 rounded-full' /> : null}
            </button>
          )
        })}
      </div>
    </div>
  )
}
