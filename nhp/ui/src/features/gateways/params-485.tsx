/**
 * Serial params sub-form for a `485` network (DOMAIN-MODEL §network `params`:
 * baud / parity / stop-bits / data-bits). Rendered by network-form.tsx when
 * `net_type === '485'`. Controlled — owns no state.
 */
import type { Net485Params } from '@/api/records'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'

const PARITY = ['none', 'even', 'odd'] as const

export function Params485({
  value,
  onChange,
}: {
  value: Net485Params
  onChange: (next: Net485Params) => void
}) {
  return (
    <div className='grid gap-4 sm:grid-cols-2'>
      <div className='grid gap-1'>
        <Label htmlFor='net-baud'>Baud</Label>
        <Input
          id='net-baud'
          type='number'
          value={value.baud}
          onChange={(e) => onChange({ ...value, baud: Number(e.target.value) })}
          placeholder='9600'
        />
      </div>
      <div className='grid gap-1'>
        <Label htmlFor='net-parity'>Parity</Label>
        <Select
          value={value.parity}
          onValueChange={(parity) =>
            onChange({ ...value, parity: parity as Net485Params['parity'] })
          }
        >
          <SelectTrigger id='net-parity'>
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {PARITY.map((p) => (
              <SelectItem key={p} value={p}>
                {p}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>
      <div className='grid gap-1'>
        <Label htmlFor='net-data'>Data bits</Label>
        <Input
          id='net-data'
          type='number'
          value={value.data_bits}
          onChange={(e) =>
            onChange({ ...value, data_bits: Number(e.target.value) })
          }
          placeholder='8'
        />
      </div>
      <div className='grid gap-1'>
        <Label htmlFor='net-stop'>Stop bits</Label>
        <Input
          id='net-stop'
          type='number'
          value={value.stop_bits}
          onChange={(e) =>
            onChange({ ...value, stop_bits: Number(e.target.value) })
          }
          placeholder='1'
        />
      </div>
    </div>
  )
}

/** Blank 485 params (common Modbus-RTU defaults). */
export function default485(): Net485Params {
  return { baud: 9600, parity: 'none', stop_bits: 1, data_bits: 8 }
}
