/**
 * The register-map editor table (ADMIN.md §1, DOMAIN-MODEL §register): a row per
 * register over the meter-type's `registers[]`. Quick fields edit inline; the full
 * register (every protocol field + the alarm ramp) edits in a side sheet. Columns
 * are toggleable so the table stays readable, and a "Group by" view clusters rows
 * under their `chart_group` so grouping is visible at a glance instead of guessed.
 * Holds working view state; the parent edit form owns the save (which bumps version).
 */
import { useMemo, useState } from 'react'
import {
  ChevronDown,
  ClipboardPaste,
  Columns3,
  Plus,
  Sparkles,
} from 'lucide-react'
import type { RegisterDef } from '@/api/records'
import { cn } from '@/lib/utils'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import {
  DropdownMenu,
  DropdownMenuCheckboxItem,
  DropdownMenuContent,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { Tabs, TabsList, TabsTrigger } from '@/components/ui/tabs'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import { BulkImportDialog } from './bulk-import-dialog'
import { blankRegister } from './register-defaults'
import {
  DEFAULT_VISIBLE_COLUMNS,
  REGISTER_COLUMNS,
  type RegisterColumnId,
} from './register-columns'
import { GroupWorkshop } from './group-workshop'
import { RegisterDetailSheet } from './register-detail-sheet'
import { RegisterRow } from './register-row'

type ViewMode = 'flat' | 'grouped' | 'workshop'

type RegisterTableProps = {
  registers: RegisterDef[]
  onChange: (registers: RegisterDef[]) => void
}

const UNGROUPED = 'Ungrouped'

export function RegisterTable({ registers, onChange }: RegisterTableProps) {
  const [bulkOpen, setBulkOpen] = useState(false)
  const [editIndex, setEditIndex] = useState<number | null>(null)
  const [editFocus, setEditFocus] = useState<'alarms' | null>(null)
  const [view, setView] = useState<ViewMode>('flat')
  const [visible, setVisible] = useState<Set<RegisterColumnId>>(
    DEFAULT_VISIBLE_COLUMNS
  )
  const [collapsed, setCollapsed] = useState<Set<string>>(new Set())

  const updateAt = (i: number, reg: RegisterDef) =>
    onChange(registers.map((r, idx) => (idx === i ? reg : r)))
  const removeAt = (i: number) =>
    onChange(registers.filter((_, idx) => idx !== i))
  const addRow = () => onChange([...registers, blankRegister(registers.length)])
  const appendImported = (defs: RegisterDef[]) =>
    onChange([...registers, ...defs])
  const openEdit = (i: number, focus?: 'alarms') => {
    setEditFocus(focus ?? null)
    setEditIndex(i)
  }

  // Distinct existing groups feed the group combobox + the grouped view.
  const groups = useMemo(
    () =>
      [...new Set(registers.map((r) => r.chart_group).filter(Boolean))].sort(),
    [registers]
  )

  const toggleColumn = (id: RegisterColumnId) =>
    setVisible((prev) => {
      const next = new Set(prev)
      if (next.has(id)) next.delete(id)
      else next.add(id)
      return next
    })

  const toggleCollapsed = (label: string) =>
    setCollapsed((prev) => {
      const next = new Set(prev)
      if (next.has(label)) next.delete(label)
      else next.add(label)
      return next
    })

  const shownColumns = REGISTER_COLUMNS.filter((c) => visible.has(c.id))
  // +2 for the always-on alarm-status and actions columns.
  const colSpan = shownColumns.length + 2

  // Keep original indices alongside each register so edits map back correctly
  // even when the grouped view reorders them visually.
  const indexed = registers.map((reg, index) => ({ reg, index }))
  const groupedRows = useMemo(() => {
    const map = new Map<string, { reg: RegisterDef; index: number }[]>()
    for (const item of indexed) {
      const label = item.reg.chart_group || UNGROUPED
      const list = map.get(label) ?? []
      list.push(item)
      map.set(label, list)
    }
    return [...map.entries()].sort(([a], [b]) =>
      a === UNGROUPED ? 1 : b === UNGROUPED ? -1 : a.localeCompare(b)
    )
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [registers])

  const renderRow = (reg: RegisterDef, index: number) => (
    <RegisterRow
      key={index}
      reg={reg}
      visible={visible}
      groups={groups}
      onChange={(r) => updateAt(index, r)}
      onRemove={() => removeAt(index)}
      onEdit={(focus) => openEdit(index, focus)}
    />
  )

  return (
    <div className='space-y-3'>
      <div className='flex flex-wrap items-center justify-between gap-2'>
        <p className='text-muted-foreground text-sm'>
          {registers.length} register{registers.length === 1 ? '' : 's'}
          {groups.length > 0 ? ` · ${groups.length} group${groups.length === 1 ? '' : 's'}` : ''}
        </p>
        <div className='flex flex-wrap items-center gap-2'>
          <Tabs value={view} onValueChange={(v) => setView(v as ViewMode)}>
            <TabsList className='h-8'>
              <TabsTrigger value='flat' className='text-xs'>
                Flat
              </TabsTrigger>
              <TabsTrigger value='grouped' className='text-xs'>
                Grouped
              </TabsTrigger>
              <TabsTrigger value='workshop' className='text-xs'>
                <Sparkles className='mr-1 size-3.5' /> Workshop
              </TabsTrigger>
            </TabsList>
          </Tabs>
          {view !== 'workshop' ? (
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button type='button' variant='outline' size='sm'>
                  <Columns3 className='mr-1 size-4' /> Columns
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align='end'>
                <DropdownMenuLabel>Visible columns</DropdownMenuLabel>
                <DropdownMenuSeparator />
                {REGISTER_COLUMNS.map((c) => (
                  <DropdownMenuCheckboxItem
                    key={c.id}
                    checked={visible.has(c.id)}
                    disabled={c.pinned}
                    onCheckedChange={() => !c.pinned && toggleColumn(c.id)}
                    onSelect={(e) => e.preventDefault()}
                  >
                    {c.label}
                  </DropdownMenuCheckboxItem>
                ))}
              </DropdownMenuContent>
            </DropdownMenu>
          ) : null}
          <Button type='button' variant='outline' size='sm' onClick={() => setBulkOpen(true)}>
            <ClipboardPaste className='mr-1 size-4' /> Bulk import
          </Button>
          <Button type='button' variant='outline' size='sm' onClick={addRow}>
            <Plus className='mr-1 size-4' /> Add register
          </Button>
        </div>
      </div>

      {view === 'workshop' ? (
        <GroupWorkshop registers={registers} onUpdate={updateAt} />
      ) : (
        <div className='overflow-x-auto rounded-md border'>
          <Table>
            <TableHeader>
              <TableRow>
                {shownColumns.map((c) => (
                  <TableHead key={c.id} className='whitespace-nowrap text-xs'>
                    {c.label}
                  </TableHead>
                ))}
                <TableHead className='text-center text-xs'>Alarm</TableHead>
                <TableHead className='text-xs' />
              </TableRow>
            </TableHeader>
            <TableBody>
              {registers.length === 0 ? (
                <TableRow>
                  <TableCell colSpan={colSpan} className='text-muted-foreground py-6 text-center text-sm'>
                    No registers yet. Add one or bulk-import.
                  </TableCell>
                </TableRow>
              ) : view === 'grouped' ? (
                groupedRows.map(([label, items]) => (
                  <GroupSection
                    key={label}
                    label={label}
                    count={items.length}
                    colSpan={colSpan}
                    collapsed={collapsed.has(label)}
                    onToggle={() => toggleCollapsed(label)}
                  >
                    {items.map(({ reg, index }) => renderRow(reg, index))}
                  </GroupSection>
                ))
              ) : (
                indexed.map(({ reg, index }) => renderRow(reg, index))
              )}
            </TableBody>
          </Table>
        </div>
      )}

      <RegisterDetailSheet
        reg={editIndex !== null ? (registers[editIndex] ?? null) : null}
        groups={groups}
        focus={editFocus}
        onChange={(r) => editIndex !== null && updateAt(editIndex, r)}
        onClose={() => setEditIndex(null)}
      />

      <BulkImportDialog
        open={bulkOpen}
        onOpenChange={setBulkOpen}
        startIndex={registers.length}
        onImport={appendImported}
      />
    </div>
  )
}

function GroupSection({
  label,
  count,
  colSpan,
  collapsed,
  onToggle,
  children,
}: {
  label: string
  count: number
  colSpan: number
  collapsed: boolean
  onToggle: () => void
  children: React.ReactNode
}) {
  return (
    <>
      <TableRow className='bg-muted/50 hover:bg-muted/50'>
        <TableCell colSpan={colSpan} className='py-1.5'>
          <button
            type='button'
            onClick={onToggle}
            className='flex items-center gap-2 text-sm font-medium'
          >
            <ChevronDown
              className={cn('size-4 transition', collapsed && '-rotate-90')}
            />
            {label}
            <Badge variant='muted'>{count}</Badge>
          </button>
        </TableCell>
      </TableRow>
      {collapsed ? null : children}
    </>
  )
}
