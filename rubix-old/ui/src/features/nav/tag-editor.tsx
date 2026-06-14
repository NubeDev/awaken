/**
 * The dashboard tag editor (docs/design/page-context-and-nav.md §3): edit a
 * board's `key → value` tag set in a popover. Tags are behaviour-affecting — they
 * feed `PageContext.tags` and the `tag` context source / `$__tag(key)` token — so
 * the save `PUT`s the whole set, and the server enforces the board's own `edit`
 * authz (a caller who cannot edit the board cannot tag it). Edits are made on a
 * working copy and persisted in one request.
 */
import { useState } from 'react'
import { Plus, Tags, Trash2 } from 'lucide-react'
import type { Uuid } from '@/api/types'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from '@/components/ui/popover'
import { useEntityTags, useReplaceEntityTags } from './use-entity-tags'
import { addRow, removeRow, setRow, toRows, toTags } from './tag-edits'

export function TagEditor({ dashboardId }: { dashboardId: Uuid }) {
  const [open, setOpen] = useState(false)
  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <Button size='sm' variant='outline'>
          <Tags className='size-3.5' /> Tags
        </Button>
      </PopoverTrigger>
      <PopoverContent align='start' className='w-80'>
        {open ? <Body dashboardId={dashboardId} onDone={() => setOpen(false)} /> : null}
      </PopoverContent>
    </Popover>
  )
}

function Body({
  dashboardId,
  onDone,
}: {
  dashboardId: Uuid
  onDone: () => void
}) {
  const { data: tags = {} } = useEntityTags('dashboard', dashboardId)
  const replace = useReplaceEntityTags('dashboard')
  const [rows, setRows] = useState(() => toRows(tags))
  const [error, setError] = useState<string | null>(null)

  const save = () => {
    replace.mutate(
      { id: dashboardId, tags: toTags(rows) },
      {
        onSuccess: onDone,
        onError: (e) => setError((e as Error).message),
      }
    )
  }

  return (
    <div className='space-y-2'>
      <p className='text-[12px] font-medium'>Dashboard tags</p>
      <p className='text-[11px] text-muted-foreground'>
        Tags drive page context — reference one in a tile as
        <span className='font-mono'> $__tag(key)</span>; values bind safely.
      </p>
      {rows.length === 0 ? (
        <p className='text-[12px] text-muted-foreground'>No tags yet.</p>
      ) : (
        rows.map((row, i) => (
          <div key={i} className='flex items-center gap-1.5'>
            <Input
              className='h-7 w-28 font-mono text-[12px]'
              value={row.key}
              onChange={(e) => setRows((rs) => setRow(rs, i, { key: e.target.value }))}
              placeholder='key'
            />
            <Input
              className='h-7 flex-1 text-[12px]'
              value={row.value}
              onChange={(e) => setRows((rs) => setRow(rs, i, { value: e.target.value }))}
              placeholder='value'
            />
            <Button
              size='icon'
              variant='ghost'
              className='size-7 text-sev-fault'
              onClick={() => setRows((rs) => removeRow(rs, i))}
              title='Remove'
            >
              <Trash2 className='size-3.5' />
            </Button>
          </div>
        ))
      )}
      <Button
        size='sm'
        variant='outline'
        className='w-full'
        onClick={() => setRows(addRow)}
      >
        <Plus className='size-3.5' /> Add tag
      </Button>
      {error ? <p className='text-[11px] text-sev-fault'>{error}</p> : null}
      <div className='flex justify-end gap-2 pt-1'>
        <Button size='sm' variant='ghost' onClick={onDone}>
          Cancel
        </Button>
        <Button size='sm' onClick={save} disabled={replace.isPending}>
          {replace.isPending ? 'Saving…' : 'Save'}
        </Button>
      </div>
    </div>
  )
}
