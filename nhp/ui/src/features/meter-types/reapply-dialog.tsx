/**
 * Per-meter "re-apply meter-type" (DOMAIN-MODEL §versioning): pick an out-of-date
 * meter, show the register diff (added/removed/changed) of the meter's current
 * registers vs the type's current `registers[]`, and on confirm re-stamp the
 * meter's registers + advance its `meter_type_version`. Never automatic — the
 * admin confirms after seeing the diff. All writes cross the gate.
 */
import { useMemo, useState } from 'react'
import type { MeterRecord, MeterTypeRecord, RegisterRec } from '@/api/records'
import { cn } from '@/lib/utils'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { useReapplyMeterType } from './hooks'
import { diffRegisters, type RegisterDiffKind } from './versioning'

type ReapplyDialogProps = {
  open: boolean
  onOpenChange: (open: boolean) => void
  type: MeterTypeRecord
  meters: MeterRecord[]
  registers: RegisterRec[]
}

const KIND_STYLE: Record<RegisterDiffKind, string> = {
  added: 'bg-emerald-500/15 text-emerald-600',
  removed: 'bg-destructive/15 text-destructive',
  changed: 'bg-amber-500/15 text-amber-600',
  unchanged: 'text-muted-foreground',
}

export function ReapplyDialog({
  open,
  onOpenChange,
  type,
  meters,
  registers,
}: ReapplyDialogProps) {
  // Meters stamped from this type, out-of-date first.
  const candidates = useMemo(
    () =>
      meters
        .filter((m) => m.content.meter_type === type.id)
        .sort(
          (a, b) =>
            a.content.meter_type_version - b.content.meter_type_version
        ),
    [meters, type.id]
  )
  const [meterId, setMeterId] = useState<string>(candidates[0]?.id ?? '')
  const meter = candidates.find((m) => m.id === meterId)
  const reapply = useReapplyMeterType()

  const meterRegisters = useMemo(
    () => (meter ? registers.filter((r) => r.content.meter === meter.id) : []),
    [registers, meter]
  )
  const diff = useMemo(
    () =>
      meter
        ? diffRegisters(
            meter.content.key,
            type.content.registers,
            meterRegisters
          )
        : [],
    [meter, type.content.registers, meterRegisters]
  )
  const changes = diff.filter((d) => d.kind !== 'unchanged').length

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className='sm:max-w-lg'>
        <DialogHeader>
          <DialogTitle>Re-apply {type.content.name}</DialogTitle>
          <DialogDescription>
            Re-stamp a meter's registers from this type's current version (
            {type.content.version}) and advance its stamped version. Existing
            history is unaffected.
          </DialogDescription>
        </DialogHeader>

        {candidates.length === 0 ? (
          <p className='text-muted-foreground text-sm'>
            No meters are stamped from this type.
          </p>
        ) : (
          <div className='space-y-4'>
            <Select value={meterId} onValueChange={setMeterId}>
              <SelectTrigger>
                <SelectValue placeholder='Select a meter' />
              </SelectTrigger>
              <SelectContent>
                {candidates.map((m) => (
                  <SelectItem key={m.id} value={m.id}>
                    {m.content.name} — v{m.content.meter_type_version}
                    {m.content.meter_type_version < type.content.version
                      ? ' (out of date)'
                      : ''}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>

            {meter ? (
              <div className='max-h-64 space-y-1 overflow-y-auto rounded-md border p-2'>
                {diff.map((d) => (
                  <div
                    key={d.key}
                    className='flex items-center justify-between text-sm'
                  >
                    <span className={cn(KIND_STYLE[d.kind], 'rounded px-1')}>
                      {d.name}
                    </span>
                    <Badge variant='outline' className='text-xs'>
                      {d.kind}
                    </Badge>
                  </div>
                ))}
              </div>
            ) : null}
          </div>
        )}

        <DialogFooter>
          <Button variant='ghost' onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button
            disabled={!meter || reapply.isPending}
            onClick={() =>
              meter &&
              reapply.mutate(
                { meter, type, meterRegisters },
                { onSuccess: () => onOpenChange(false) }
              )
            }
          >
            {reapply.isPending
              ? 'Applying…'
              : `Apply (${changes} change${changes === 1 ? '' : 's'})`}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
