/**
 * Per-register alarm threshold ramp editor (DOMAIN-MODEL §Alarms): the
 * `{ thresholds: [{ value, severity }], for }` shape that both colours the chart
 * and fires the rule. Edited inline against the register's `alarm` field; clearing
 * all thresholds removes the alarm.
 */
import { Plus, Trash2 } from 'lucide-react'
import type { Alarm, AlarmSeverity, AlarmThreshold } from '@/api/records'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'

const SEVERITIES: AlarmSeverity[] = ['ok', 'warning', 'critical']

type AlarmEditorProps = {
  alarm: Alarm | undefined
  onChange: (alarm: Alarm | undefined) => void
}

export function AlarmEditor({ alarm, onChange }: AlarmEditorProps) {
  const thresholds = alarm?.thresholds ?? []

  const setThresholds = (next: AlarmThreshold[]) => {
    if (next.length === 0) {
      onChange(undefined)
      return
    }
    onChange({ thresholds: next, for: alarm?.for })
  }

  const updateRow = (i: number, patch: Partial<AlarmThreshold>) => {
    setThresholds(thresholds.map((t, idx) => (idx === i ? { ...t, ...patch } : t)))
  }

  const addRow = () =>
    setThresholds([...thresholds, { value: 0, severity: 'warning' }])

  return (
    <div className='space-y-3'>
      {thresholds.length === 0 ? (
        <p className='text-muted-foreground text-sm'>
          No alarm thresholds. Add a step to colour the chart and fire the rule.
        </p>
      ) : (
        <div className='space-y-2'>
          {thresholds.map((t, i) => (
            <div key={i} className='flex items-end gap-2'>
              <div className='grid gap-1'>
                <Label className='text-xs'>Value (≥)</Label>
                <Input
                  className='w-28'
                  type='number'
                  placeholder='baseline'
                  value={t.value ?? ''}
                  onChange={(e) =>
                    updateRow(i, {
                      value: e.target.value === '' ? null : Number(e.target.value),
                    })
                  }
                />
              </div>
              <div className='grid gap-1'>
                <Label className='text-xs'>Severity</Label>
                <Select
                  value={t.severity}
                  onValueChange={(v) =>
                    updateRow(i, { severity: v as AlarmSeverity })
                  }
                >
                  <SelectTrigger className='w-32'>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {SEVERITIES.map((s) => (
                      <SelectItem key={s} value={s}>
                        {s}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
              <Button
                type='button'
                variant='ghost'
                size='icon'
                onClick={() =>
                  setThresholds(thresholds.filter((_, idx) => idx !== i))
                }
              >
                <Trash2 className='size-4' />
              </Button>
            </div>
          ))}
        </div>
      )}

      <div className='flex items-end gap-2'>
        <Button type='button' variant='outline' size='sm' onClick={addRow}>
          <Plus className='mr-1 size-4' /> Add threshold
        </Button>
        {thresholds.length > 0 ? (
          <div className='grid gap-1'>
            <Label className='text-xs'>Dwell (for)</Label>
            <Input
              className='w-24'
              placeholder='5m'
              value={alarm?.for ?? ''}
              onChange={(e) =>
                onChange({
                  thresholds,
                  for: e.target.value === '' ? undefined : e.target.value,
                })
              }
            />
          </div>
        ) : null}
      </div>
    </div>
  )
}
