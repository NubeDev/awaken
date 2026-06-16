/**
 * TCP params sub-form for an `ethernet` network (DOMAIN-MODEL §network `params`:
 * ip / port). Rendered by network-form.tsx when `net_type === 'ethernet'`.
 * Controlled — owns no state.
 */
import type { NetEthernetParams } from '@/api/records'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'

export function ParamsEthernet({
  value,
  onChange,
}: {
  value: NetEthernetParams
  onChange: (next: NetEthernetParams) => void
}) {
  return (
    <div className='grid gap-4 sm:grid-cols-2'>
      <div className='grid gap-1'>
        <Label htmlFor='net-ip'>IP address</Label>
        <Input
          id='net-ip'
          value={value.ip}
          onChange={(e) => onChange({ ...value, ip: e.target.value })}
          placeholder='192.168.1.50'
        />
      </div>
      <div className='grid gap-1'>
        <Label htmlFor='net-port'>Port</Label>
        <Input
          id='net-port'
          type='number'
          value={value.port}
          onChange={(e) => onChange({ ...value, port: Number(e.target.value) })}
          placeholder='502'
        />
      </div>
    </div>
  )
}

/** Blank ethernet params (Modbus-TCP default port). */
export function defaultEthernet(): NetEthernetParams {
  return { ip: '', port: 502 }
}
