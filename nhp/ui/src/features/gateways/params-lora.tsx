/**
 * Radio params sub-form for a `lora` (LoRaWAN) network (DOMAIN-MODEL §network
 * `params`: region / spreading_factor). Rendered by network-form.tsx when
 * `net_type === 'lora'`. Controlled — owns no state.
 */
import type { NetLoraParams } from '@/api/records'
import { Label } from '@/components/ui/label'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'

// The common LoRaWAN regional frequency plans. Free-form on the wire; this is the
// UI convenience list (rubix has no enum FieldType — OVERVIEW gap #1).
const REGIONS = ['AU915', 'US915', 'EU868', 'AS923', 'IN865'] as const
// SF7 (fast/short) .. SF12 (slow/long-range).
const SPREADING_FACTORS = [7, 8, 9, 10, 11, 12] as const

export function ParamsLora({
  value,
  onChange,
}: {
  value: NetLoraParams
  onChange: (next: NetLoraParams) => void
}) {
  return (
    <div className='grid gap-4 sm:grid-cols-2'>
      <div className='grid gap-1'>
        <Label htmlFor='net-region'>Region</Label>
        <Select
          value={value.region}
          onValueChange={(region) => onChange({ ...value, region })}
        >
          <SelectTrigger id='net-region'>
            <SelectValue placeholder='AU915' />
          </SelectTrigger>
          <SelectContent>
            {REGIONS.map((r) => (
              <SelectItem key={r} value={r}>
                {r}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>
      <div className='grid gap-1'>
        <Label htmlFor='net-sf'>Spreading factor</Label>
        <Select
          value={String(value.spreading_factor)}
          onValueChange={(sf) =>
            onChange({ ...value, spreading_factor: Number(sf) })
          }
        >
          <SelectTrigger id='net-sf'>
            <SelectValue placeholder='SF10' />
          </SelectTrigger>
          <SelectContent>
            {SPREADING_FACTORS.map((sf) => (
              <SelectItem key={sf} value={String(sf)}>
                SF{sf}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>
    </div>
  )
}

/** Blank LoRa params (AU915 plan, mid-range SF10). */
export function defaultLora(): NetLoraParams {
  return { region: 'AU915', spreading_factor: 10 }
}
