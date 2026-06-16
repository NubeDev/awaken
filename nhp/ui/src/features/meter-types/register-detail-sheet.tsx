/**
 * Full-register editor in a side sheet (DOMAIN-MODEL §register). The inline table
 * stays compact; deep editing — every protocol field plus the roomy alarm ramp —
 * happens here with space to breathe, grouped into Identity / Modbus / Presentation
 * / Alarms sections. Controlled: edits flow straight back through `onChange`.
 */
import { useEffect, useRef } from 'react'
import { Bell } from 'lucide-react'
import type { Alarm, RegisterDef } from '@/api/records'
import {
  BYTE_ORDER,
  CHART_TYPE,
  DATATYPE,
  FN_CODE,
  toOptions,
} from '@/enums/options'
import { Checkbox } from '@/components/ui/checkbox'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { Separator } from '@/components/ui/separator'
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
} from '@/components/ui/sheet'
import { AlarmEditor } from './alarm-editor'
import { GroupCombobox } from './group-combobox'

type RegisterDetailSheetProps = {
  /** The register to edit, or null when the sheet is closed. */
  reg: RegisterDef | null
  /** Distinct groups across sibling registers, for the group combobox. */
  groups: string[]
  /** When opened, scroll straight to this section (e.g. clicking the alarm badge). */
  focus?: 'alarms' | null
  onChange: (reg: RegisterDef) => void
  onClose: () => void
}

function Field({
  label,
  hint,
  children,
}: {
  label: string
  hint?: string
  children: React.ReactNode
}) {
  return (
    <div className='grid gap-1.5'>
      <Label className='text-xs'>{label}</Label>
      {children}
      {hint ? <p className='text-muted-foreground text-xs'>{hint}</p> : null}
    </div>
  )
}

function EnumField({
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
      <SelectTrigger className='h-9'>
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

export function RegisterDetailSheet({
  reg,
  groups,
  focus,
  onChange,
  onClose,
}: RegisterDetailSheetProps) {
  const open = reg !== null
  const set = (patch: Partial<RegisterDef>) => reg && onChange({ ...reg, ...patch })
  const num = (v: string) => (v === '' ? 0 : Number(v))
  const alarmsRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    if (open && focus === 'alarms') {
      // After the sheet's open animation, bring the alarm section into view.
      const t = setTimeout(
        () => alarmsRef.current?.scrollIntoView({ behavior: 'smooth', block: 'start' }),
        250
      )
      return () => clearTimeout(t)
    }
  }, [open, focus, reg])

  return (
    <Sheet open={open} onOpenChange={(o) => (o ? null : onClose())}>
      <SheetContent className='w-full gap-0 overflow-y-auto sm:max-w-xl'>
        {reg ? (
          <>
            <SheetHeader>
              <SheetTitle>{reg.name || reg.key || 'Register'}</SheetTitle>
              <SheetDescription>
                Edit every field for this register, including alarm thresholds.
              </SheetDescription>
            </SheetHeader>

            <div className='space-y-6 px-4 pb-8'>
              <section className='space-y-3'>
                <h4 className='text-sm font-medium'>Identity</h4>
                <div className='grid grid-cols-2 gap-3'>
                  <Field label='Key'>
                    <Input
                      className='h-9'
                      value={reg.key}
                      onChange={(e) => set({ key: e.target.value })}
                    />
                  </Field>
                  <Field label='Name'>
                    <Input
                      className='h-9'
                      value={reg.name}
                      onChange={(e) => set({ name: e.target.value })}
                    />
                  </Field>
                </div>
              </section>

              <Separator />

              <section className='space-y-3'>
                <h4 className='text-sm font-medium'>Modbus</h4>
                <div className='grid grid-cols-2 gap-3'>
                  <Field label='Address'>
                    <Input
                      className='h-9'
                      type='number'
                      value={reg.address}
                      onChange={(e) => set({ address: num(e.target.value) })}
                    />
                  </Field>
                  <Field label='Function code'>
                    <EnumField
                      value={reg.fn_code}
                      values={FN_CODE}
                      onChange={(v) => set({ fn_code: v as RegisterDef['fn_code'] })}
                    />
                  </Field>
                  <Field label='Datatype'>
                    <EnumField
                      value={reg.datatype}
                      values={DATATYPE}
                      onChange={(v) => set({ datatype: v as RegisterDef['datatype'] })}
                    />
                  </Field>
                  <Field label='Word count'>
                    <Input
                      className='h-9'
                      type='number'
                      value={reg.word_count}
                      onChange={(e) => set({ word_count: num(e.target.value) })}
                    />
                  </Field>
                  <Field label='Byte order'>
                    <EnumField
                      value={reg.byte_order}
                      values={BYTE_ORDER}
                      onChange={(v) => set({ byte_order: v as RegisterDef['byte_order'] })}
                    />
                  </Field>
                  <Field label='Signed'>
                    <div className='flex h-9 items-center'>
                      <Checkbox
                        checked={reg.signed}
                        onCheckedChange={(c) => set({ signed: Boolean(c) })}
                      />
                    </div>
                  </Field>
                  <Field label='Scale' hint='raw × scale + offset'>
                    <Input
                      className='h-9'
                      type='number'
                      value={reg.scale}
                      onChange={(e) => set({ scale: num(e.target.value) })}
                    />
                  </Field>
                  <Field label='Offset'>
                    <Input
                      className='h-9'
                      type='number'
                      value={reg.offset}
                      onChange={(e) => set({ offset: num(e.target.value) })}
                    />
                  </Field>
                </div>
              </section>

              <Separator />

              <section className='space-y-3'>
                <h4 className='text-sm font-medium'>Presentation</h4>
                <div className='grid grid-cols-2 gap-3'>
                  <Field label='Unit'>
                    <Input
                      className='h-9'
                      value={reg.unit}
                      onChange={(e) => set({ unit: e.target.value })}
                    />
                  </Field>
                  <Field label='Quantity'>
                    <Input
                      className='h-9'
                      value={reg.quantity}
                      onChange={(e) => set({ quantity: e.target.value })}
                    />
                  </Field>
                  <Field label='Chart type'>
                    <EnumField
                      value={reg.chart_type}
                      values={CHART_TYPE}
                      onChange={(v) => set({ chart_type: v as RegisterDef['chart_type'] })}
                    />
                  </Field>
                  <Field label='Group' hint='Charts stack registers sharing a group.'>
                    <GroupCombobox
                      value={reg.chart_group}
                      options={groups}
                      onChange={(g) => set({ chart_group: g })}
                      className='w-full'
                    />
                  </Field>
                  <Field label='Precision'>
                    <Input
                      className='h-9'
                      type='number'
                      value={reg.precision}
                      onChange={(e) => set({ precision: num(e.target.value) })}
                    />
                  </Field>
                  <Field label='History'>
                    <div className='flex h-9 items-center gap-2'>
                      <Checkbox
                        checked={reg.history}
                        onCheckedChange={(c) => set({ history: Boolean(c) })}
                      />
                      <span className='text-muted-foreground text-xs'>
                        Store time-series
                      </span>
                    </div>
                  </Field>
                </div>
              </section>

              <Separator />

              <section ref={alarmsRef} className='scroll-mt-4 space-y-3'>
                <h4 className='flex items-center gap-2 text-sm font-medium'>
                  <Bell className='size-4' /> Alarms
                </h4>
                <AlarmEditor
                  alarm={reg.alarm}
                  onChange={(alarm: Alarm | undefined) => set({ alarm })}
                />
              </section>
            </div>
          </>
        ) : null}
      </SheetContent>
    </Sheet>
  )
}
