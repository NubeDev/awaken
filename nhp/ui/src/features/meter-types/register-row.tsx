/**
 * One editable register row in the register-map editor (DOMAIN-MODEL §register).
 * Every field is editable inline; enum fields are dropdowns sourced from the
 * shared enum options (enums/options.ts). The alarm ramp expands below the row.
 */
import { useState } from 'react'
import { Bell, BellRing, ChevronDown, Trash2 } from 'lucide-react'
import type { Alarm, RegisterDef } from '@/api/records'
import {
  BYTE_ORDER,
  CHART_TYPE,
  DATATYPE,
  FN_CODE,
  toOptions,
} from '@/enums/options'
import { cn } from '@/lib/utils'
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
import { AlarmEditor } from './alarm-editor'

type RegisterRowProps = {
  reg: RegisterDef
  onChange: (reg: RegisterDef) => void
  onRemove: () => void
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

export function RegisterRow({ reg, onChange, onRemove }: RegisterRowProps) {
  const [open, setOpen] = useState(false)
  const set = (patch: Partial<RegisterDef>) => onChange({ ...reg, ...patch })
  const num = (v: string) => (v === '' ? 0 : Number(v))
  const hasAlarm = (reg.alarm?.thresholds?.length ?? 0) > 0

  return (
    <>
      <TableRow>
        <TableCell>
          <TextCell value={reg.key} onChange={(v) => set({ key: v })} className='w-32' />
        </TableCell>
        <TableCell>
          <TextCell value={reg.name} onChange={(v) => set({ name: v })} className='w-36' />
        </TableCell>
        <TableCell>
          <TextCell
            type='number'
            value={reg.address}
            onChange={(v) => set({ address: num(v) })}
            className='w-20'
          />
        </TableCell>
        <TableCell className='w-36'>
          <EnumCell value={reg.fn_code} values={FN_CODE} onChange={(v) => set({ fn_code: v as RegisterDef['fn_code'] })} />
        </TableCell>
        <TableCell className='w-28'>
          <EnumCell value={reg.datatype} values={DATATYPE} onChange={(v) => set({ datatype: v as RegisterDef['datatype'] })} />
        </TableCell>
        <TableCell>
          <TextCell
            type='number'
            value={reg.word_count}
            onChange={(v) => set({ word_count: num(v) })}
            className='w-16'
          />
        </TableCell>
        <TableCell className='w-32'>
          <EnumCell value={reg.byte_order} values={BYTE_ORDER} onChange={(v) => set({ byte_order: v as RegisterDef['byte_order'] })} />
        </TableCell>
        <TableCell>
          <TextCell type='number' value={reg.scale} onChange={(v) => set({ scale: num(v) })} className='w-16' />
        </TableCell>
        <TableCell>
          <TextCell type='number' value={reg.offset} onChange={(v) => set({ offset: num(v) })} className='w-16' />
        </TableCell>
        <TableCell className='text-center'>
          <Checkbox checked={reg.signed} onCheckedChange={(c) => set({ signed: Boolean(c) })} />
        </TableCell>
        <TableCell>
          <TextCell value={reg.unit} onChange={(v) => set({ unit: v })} className='w-16' />
        </TableCell>
        <TableCell>
          <TextCell value={reg.quantity} onChange={(v) => set({ quantity: v })} className='w-24' />
        </TableCell>
        <TableCell className='text-center'>
          <Checkbox checked={reg.history} onCheckedChange={(c) => set({ history: Boolean(c) })} />
        </TableCell>
        <TableCell className='w-24'>
          <EnumCell value={reg.chart_type} values={CHART_TYPE} onChange={(v) => set({ chart_type: v as RegisterDef['chart_type'] })} />
        </TableCell>
        <TableCell>
          <TextCell value={reg.chart_group} onChange={(v) => set({ chart_group: v })} className='w-24' />
        </TableCell>
        <TableCell>
          <TextCell type='number' value={reg.precision} onChange={(v) => set({ precision: num(v) })} className='w-16' />
        </TableCell>
        <TableCell>
          <div className='flex items-center gap-1'>
            <Button
              type='button'
              variant='ghost'
              size='icon'
              title='Edit alarm thresholds'
              onClick={() => setOpen((o) => !o)}
            >
              {hasAlarm ? <BellRing className='size-4 text-amber-500' /> : <Bell className='size-4' />}
              <ChevronDown className={cn('size-3 transition', open && 'rotate-180')} />
            </Button>
            <Button type='button' variant='ghost' size='icon' onClick={onRemove}>
              <Trash2 className='size-4' />
            </Button>
          </div>
        </TableCell>
      </TableRow>
      {open ? (
        <TableRow>
          <TableCell colSpan={17} className='bg-muted/40'>
            <div className='p-2'>
              <p className='mb-2 text-sm font-medium'>
                Alarm thresholds — {reg.name}
              </p>
              <AlarmEditor
                alarm={reg.alarm}
                onChange={(alarm: Alarm | undefined) => set({ alarm })}
              />
            </div>
          </TableCell>
        </TableRow>
      ) : null}
    </>
  )
}
