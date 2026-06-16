/**
 * One register row in the register-map editor (DOMAIN-MODEL §register). Only the
 * columns enabled in `visible` render, so the table stays readable; the alarm and
 * group show as compact status badges. Inline editing is kept for the quick fields;
 * the "Edit" affordance opens the detail sheet for everything (and the full alarm
 * ramp). `key` is pinned so a row is never anonymous.
 */
import { Bell, Pencil, Trash2 } from 'lucide-react'
import type { RegisterDef } from '@/api/records'
import {
  BYTE_ORDER,
  CHART_TYPE,
  DATATYPE,
  FN_CODE,
  toOptions,
} from '@/enums/options'
import { cn } from '@/lib/utils'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { Input } from '@/components/ui/input'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { TableCell, TableRow } from '@/components/ui/table'
import { GroupCombobox } from './group-combobox'
import type { RegisterColumnId } from './register-columns'

type RegisterRowProps = {
  reg: RegisterDef
  visible: Set<RegisterColumnId>
  groups: string[]
  onChange: (reg: RegisterDef) => void
  onRemove: () => void
  /** Open the detail sheet; `focus` scrolls straight to a section. */
  onEdit: (focus?: 'alarms') => void
}

function EnumCell({
  value,
  values,
  onChange,
}: {
  value: string
  values: readonly string[]
  onChange: (v: string) => void
}) {
  return (
    <Select value={value} onValueChange={onChange}>
      <SelectTrigger className='h-8 w-full'>
        <SelectValue />
      </SelectTrigger>
      <SelectContent>
        {toOptions(values).map((o) => (
          <SelectItem key={o.value} value={o.value}>
            {o.label}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  )
}

function TextCell({
  value,
  onChange,
  className,
  type = 'text',
}: {
  value: string | number
  onChange: (v: string) => void
  className?: string
  type?: string
}) {
  return (
    <Input
      type={type}
      className={cn('h-8', className)}
      value={value}
      onChange={(e) => onChange(e.target.value)}
    />
  )
}

export function RegisterRow({
  reg,
  visible,
  groups,
  onChange,
  onRemove,
  onEdit,
}: RegisterRowProps) {
  const set = (patch: Partial<RegisterDef>) => onChange({ ...reg, ...patch })
  const num = (v: string) => (v === '' ? 0 : Number(v))
  const alarmCount = reg.alarm?.thresholds?.length ?? 0
  const has = (id: RegisterColumnId) => visible.has(id)
  const severity = reg.alarm?.thresholds?.some((t) => t.severity === 'critical')
    ? 'fault'
    : reg.alarm?.thresholds?.some((t) => t.severity === 'warning')
      ? 'warning'
      : 'muted'

  return (
    <TableRow>
      {has('key') && (
        <TableCell>
          <TextCell value={reg.key} onChange={(v) => set({ key: v })} className='w-32' />
        </TableCell>
      )}
      {has('name') && (
        <TableCell>
          <TextCell value={reg.name} onChange={(v) => set({ name: v })} className='w-36' />
        </TableCell>
      )}
      {has('address') && (
        <TableCell>
          <TextCell
            type='number'
            value={reg.address}
            onChange={(v) => set({ address: num(v) })}
            className='w-20'
          />
        </TableCell>
      )}
      {has('fn_code') && (
        <TableCell className='w-36'>
          <EnumCell value={reg.fn_code} values={FN_CODE} onChange={(v) => set({ fn_code: v as RegisterDef['fn_code'] })} />
        </TableCell>
      )}
      {has('datatype') && (
        <TableCell className='w-28'>
          <EnumCell value={reg.datatype} values={DATATYPE} onChange={(v) => set({ datatype: v as RegisterDef['datatype'] })} />
        </TableCell>
      )}
      {has('word_count') && (
        <TableCell>
          <TextCell
            type='number'
            value={reg.word_count}
            onChange={(v) => set({ word_count: num(v) })}
            className='w-16'
          />
        </TableCell>
      )}
      {has('byte_order') && (
        <TableCell className='w-32'>
          <EnumCell value={reg.byte_order} values={BYTE_ORDER} onChange={(v) => set({ byte_order: v as RegisterDef['byte_order'] })} />
        </TableCell>
      )}
      {has('scale') && (
        <TableCell>
          <TextCell type='number' value={reg.scale} onChange={(v) => set({ scale: num(v) })} className='w-16' />
        </TableCell>
      )}
      {has('offset') && (
        <TableCell>
          <TextCell type='number' value={reg.offset} onChange={(v) => set({ offset: num(v) })} className='w-16' />
        </TableCell>
      )}
      {has('signed') && (
        <TableCell className='text-center'>
          <Checkbox checked={reg.signed} onCheckedChange={(c) => set({ signed: Boolean(c) })} />
        </TableCell>
      )}
      {has('unit') && (
        <TableCell>
          <TextCell value={reg.unit} onChange={(v) => set({ unit: v })} className='w-16' />
        </TableCell>
      )}
      {has('quantity') && (
        <TableCell>
          <TextCell value={reg.quantity} onChange={(v) => set({ quantity: v })} className='w-24' />
        </TableCell>
      )}
      {has('history') && (
        <TableCell className='text-center'>
          <Checkbox checked={reg.history} onCheckedChange={(c) => set({ history: Boolean(c) })} />
        </TableCell>
      )}
      {has('chart_type') && (
        <TableCell className='w-24'>
          <EnumCell value={reg.chart_type} values={CHART_TYPE} onChange={(v) => set({ chart_type: v as RegisterDef['chart_type'] })} />
        </TableCell>
      )}
      {has('chart_group') && (
        <TableCell>
          <GroupCombobox
            value={reg.chart_group}
            options={groups}
            onChange={(g) => set({ chart_group: g })}
            className='w-32'
          />
        </TableCell>
      )}
      {has('precision') && (
        <TableCell>
          <TextCell type='number' value={reg.precision} onChange={(v) => set({ precision: num(v) })} className='w-16' />
        </TableCell>
      )}
      {/* Alarm status badge — always shown, opens the detail sheet to edit */}
      <TableCell className='text-center'>
        <button type='button' onClick={() => onEdit('alarms')} title='Edit alarm thresholds'>
          {alarmCount > 0 ? (
            <Badge variant={severity}>
              <Bell className='size-3' />
              {alarmCount}
            </Badge>
          ) : (
            <Badge variant='outline' className='text-muted-foreground'>
              <Bell className='size-3' />
            </Badge>
          )}
        </button>
      </TableCell>
      <TableCell>
        <div className='flex items-center gap-1'>
          <Button type='button' variant='ghost' size='icon' title='Edit register' onClick={() => onEdit()}>
            <Pencil className='size-4' />
          </Button>
          <Button type='button' variant='ghost' size='icon' title='Delete register' onClick={onRemove}>
            <Trash2 className='size-4' />
          </Button>
        </div>
      </TableCell>
    </TableRow>
  )
}
