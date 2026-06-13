import { useState } from 'react'
import { Building2, Pencil, Plus, Search, Trash2 } from 'lucide-react'
import { useDeleteEquip } from '@/api/hooks'
import { tagNames } from '@/api/tags'
import type { Equip, Site, Uuid } from '@/api/types'
import { EquipKindIcon } from '@/lib/equip-icon'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { ConfirmDialog } from '@/components/confirm-dialog'
import { EquipFormDialog } from './equip-form-dialog'

type EquipBrowserProps = {
  site: Site | undefined
  equips: Equip[]
  faultEquips: Set<Uuid | undefined>
  activeId: Uuid | undefined
  onSelect: (id: Uuid) => void
}

/** Left pane: filterable equipment tree for the active site. */
export function EquipBrowser({
  site,
  equips,
  faultEquips,
  activeId,
  onSelect,
}: EquipBrowserProps) {
  const [filter, setFilter] = useState('')
  const [createOpen, setCreateOpen] = useState(false)
  const [editEquip, setEditEquip] = useState<Equip | null>(null)
  const [deleteEquip, setDeleteEquip] = useState<Equip | null>(null)
  const del = useDeleteEquip()
  const q = filter.trim().toLowerCase()
  const visible = q
    ? equips.filter(
        (e) =>
          e.display_name.toLowerCase().includes(q) ||
          tagNames(e.tags).some(
            (t) => `#${t}`.includes(q) || t.includes(q.replace(/^#/, ''))
          )
      )
    : equips

  return (
    <div className='flex h-full flex-col gap-2'>
      <div className='relative'>
        <Search className='absolute start-2.5 top-1/2 size-3.5 -translate-y-1/2 text-muted-foreground' />
        <Input
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
          placeholder='Filter equipment or #tag'
          className='h-8 ps-8 text-[12.5px]'
        />
      </div>
      <div className='flex items-center justify-between px-1.5 pt-1'>
        <div className='flex items-center gap-2 text-[11.5px] font-medium text-muted-foreground'>
          <Building2 className='size-3.5' />
          {site?.display_name ?? '…'}
        </div>
        {site ? (
          <Button
            size='icon'
            variant='ghost'
            className='size-6'
            title='Add equipment'
            onClick={() => setCreateOpen(true)}
          >
            <Plus className='size-3.5' />
          </Button>
        ) : null}
      </div>
      <div className='flex flex-col gap-0.5'>
        {visible.map((e) => {
          const fault = faultEquips.has(e.id)
          const active = e.id === activeId
          return (
            <div
              key={e.id}
              className={cn(
                'group flex w-full items-center gap-1 rounded-md pe-1 transition-colors hover:bg-accent',
                active && 'bg-accent font-medium'
              )}
            >
              <button
                onClick={() => onSelect(e.id)}
                className='flex min-w-0 flex-1 items-center gap-2.5 px-2 py-[7px] text-left text-[12.5px]'
              >
                <EquipKindIcon
                  tags={e.tags}
                  className={cn(
                    'size-4 shrink-0',
                    active ? 'text-primary' : 'text-muted-foreground'
                  )}
                />
                <span className='flex-1 truncate'>{e.display_name}</span>
                {fault ? (
                  <span className='size-1.5 shrink-0 rounded-full bg-sev-fault' />
                ) : null}
              </button>
              <div className='flex shrink-0 items-center opacity-0 transition-opacity group-hover:opacity-100'>
                <Button
                  size='icon'
                  variant='ghost'
                  className='size-6'
                  title='Edit equip'
                  onClick={() => setEditEquip(e)}
                >
                  <Pencil className='size-3' />
                </Button>
                <Button
                  size='icon'
                  variant='ghost'
                  className='size-6 text-sev-fault'
                  title='Delete equip'
                  onClick={() => setDeleteEquip(e)}
                >
                  <Trash2 className='size-3' />
                </Button>
              </div>
            </div>
          )
        })}
      </div>

      {site ? (
        <EquipFormDialog
          mode='create'
          siteId={site.id}
          open={createOpen}
          onOpenChange={setCreateOpen}
        />
      ) : null}
      {editEquip ? (
        <EquipFormDialog
          mode='edit'
          equip={editEquip}
          open={editEquip !== null}
          onOpenChange={(o) => !o && setEditEquip(null)}
        />
      ) : null}
      <ConfirmDialog
        open={deleteEquip !== null}
        onOpenChange={(o) => !o && setDeleteEquip(null)}
        destructive
        title={`Delete ${deleteEquip?.display_name ?? 'equip'}?`}
        desc={
          <>
            This deletes the equip and <strong>cascades</strong> to all its
            points and their history. This cannot be undone.
          </>
        }
        confirmText='Delete equip'
        isLoading={del.isPending}
        handleConfirm={() => {
          if (deleteEquip)
            del.mutate(deleteEquip.id, {
              onSuccess: () => setDeleteEquip(null),
            })
        }}
      />
    </div>
  )
}
