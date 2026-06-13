import { useState } from 'react'
import { Building, Building2, Pencil, Plus, Trash2 } from 'lucide-react'
import { useDeleteDashboard } from '@/api/hooks'
import type { Dashboard } from '@/api/types'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { ConfirmDialog } from '@/components/confirm-dialog'

type DashboardPickerProps = {
  dashboards: Dashboard[]
  selectedId: string | undefined
  onSelect: (id: string) => void
  onNew: () => void
  onEdit: (dashboard: Dashboard) => void
}

/**
 * Header strip: choose the active dashboard, create a new one, or edit/delete
 * the current one. An overview board (no site) is flagged so the operator knows
 * it spans the whole org, not one site.
 */
export function DashboardPicker({
  dashboards,
  selectedId,
  onSelect,
  onNew,
  onEdit,
}: DashboardPickerProps) {
  const del = useDeleteDashboard()
  const [confirmOpen, setConfirmOpen] = useState(false)
  const selected = dashboards.find((d) => d.id === selectedId)

  return (
    <div className='flex items-center gap-2'>
      <Select value={selectedId} onValueChange={onSelect}>
        <SelectTrigger size='sm' className='w-[260px]'>
          <SelectValue placeholder='Select a dashboard' />
        </SelectTrigger>
        <SelectContent>
          {dashboards.map((d) => (
            <SelectItem key={d.id} value={d.id}>
              <span className='flex items-center gap-2'>
                {d.site_id ? (
                  <Building2 className='size-3.5' />
                ) : (
                  <Building className='size-3.5 text-primary' />
                )}
                {d.title}
              </span>
            </SelectItem>
          ))}
        </SelectContent>
      </Select>

      {selected && !selected.site_id ? (
        <Badge variant='muted' className='h-5 gap-1 px-1.5 text-[10px]'>
          <Building className='size-3' /> org overview
        </Badge>
      ) : null}

      <div className='ms-auto flex items-center gap-1'>
        {selected ? (
          <>
            <Button
              size='sm'
              variant='ghost'
              onClick={() => onEdit(selected)}
              title='Rename dashboard'
            >
              <Pencil className='size-3.5' /> Rename
            </Button>
            <Button
              size='icon'
              variant='ghost'
              className='size-8 text-sev-fault'
              title='Delete dashboard'
              onClick={() => setConfirmOpen(true)}
            >
              <Trash2 className='size-3.5' />
            </Button>
          </>
        ) : null}
        <Button size='sm' onClick={onNew}>
          <Plus className='size-4' /> New
        </Button>
      </div>

      <ConfirmDialog
        open={confirmOpen}
        onOpenChange={setConfirmOpen}
        destructive
        title={`Delete "${selected?.title}"?`}
        desc='This deletes the dashboard and all its tiles. The underlying points and boards are untouched. This cannot be undone.'
        confirmText='Delete dashboard'
        isLoading={del.isPending}
        handleConfirm={() => {
          if (selected)
            del.mutate(selected.id, { onSuccess: () => setConfirmOpen(false) })
        }}
      />
    </div>
  )
}
