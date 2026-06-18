/**
 * Step 2 of the gateway wizard — THE bulk-networks step (WS-06 task 2): generate
 * N networks from a count + net_type/protocol/max_devices + a naming pattern
 * (`gw-01-net-{n}`) + per-type params defaults. Reuses the 485/ethernet params
 * sub-forms from the admin gateways feature (no reinvention). A live preview of
 * the generated keys reassures the operator before the final preview/write.
 */
import type {
  Net485Params,
  NetEthernetParams,
  NetLoraParams,
  NetParams,
} from '@/api/records'
import { NET_TYPE, type NetType, type Protocol } from '@/enums/options'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { default485, Params485 } from '@/features/gateways/params-485'
import {
  defaultEthernet,
  ParamsEthernet,
} from '@/features/gateways/params-ethernet'
import { defaultLora, ParamsLora } from '@/features/gateways/params-lora'
import { networkKeys } from './plan'

export interface NetworksStepValue {
  count: number
  netType: NetType
  protocol: Protocol
  maxDevices: number
  namePattern: string
  p485: Net485Params
  pEth: NetEthernetParams
  pLora: NetLoraParams
}

export function defaultNetworksStep(gatewayKey: string): NetworksStepValue {
  return {
    count: 30,
    netType: '485',
    protocol: 'modbus',
    maxDevices: 16,
    namePattern: `${gatewayKey || 'gw-01'}-net-{n}`,
    p485: default485(),
    pEth: defaultEthernet(),
    pLora: defaultLora(),
  }
}

/** Resolve the active params shape for the plan. */
export function networksParams(v: NetworksStepValue): NetParams {
  return v.netType === '485' ? v.p485 : v.netType === 'lora' ? v.pLora : v.pEth
}

const NET_TYPE_LABEL: Record<NetType, string> = {
  '485': 'RS-485 (serial)',
  ethernet: 'Ethernet (TCP)',
  lora: 'LoRaWAN (radio)',
}

// The protocol each transport carries (a LoRa radio speaks LoRaWAN, the field
// buses speak Modbus) — re-pinned when net_type changes so they never disagree.
const PROTOCOL_FOR: Record<NetType, Protocol> = {
  '485': 'modbus',
  ethernet: 'modbus',
  lora: 'lora',
}

export function NetworksStep({
  value,
  onChange,
}: {
  value: NetworksStepValue
  onChange: (next: NetworksStepValue) => void
}) {
  const keys = networkKeys({
    count: Number.isFinite(value.count) ? value.count : 0,
    netType: value.netType,
    protocol: value.protocol,
    maxDevices: value.maxDevices,
    namePattern: value.namePattern,
    params: networksParams(value),
  })

  return (
    <div className='grid gap-4'>
      <div className='grid gap-4 sm:grid-cols-2'>
        <div className='grid gap-1'>
          <Label htmlFor='net-count'>How many networks</Label>
          <Input
            id='net-count'
            type='number'
            min={1}
            max={200}
            value={value.count}
            onChange={(e) =>
              onChange({ ...value, count: Number(e.target.value) })
            }
            placeholder='30'
          />
        </div>
        <div className='grid gap-1'>
          <Label htmlFor='net-pattern'>Naming pattern</Label>
          <Input
            id='net-pattern'
            value={value.namePattern}
            onChange={(e) =>
              onChange({ ...value, namePattern: e.target.value })
            }
            placeholder='gw-01-net-{n}'
          />
        </div>
      </div>

      <div className='grid gap-4 sm:grid-cols-3'>
        <div className='grid gap-1'>
          <Label htmlFor='net-type'>Type</Label>
          <Select
            value={value.netType}
            onValueChange={(v) =>
              onChange({
                ...value,
                netType: v as NetType,
                protocol: PROTOCOL_FOR[v as NetType],
              })
            }
          >
            <SelectTrigger id='net-type'>
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {NET_TYPE.map((t) => (
                <SelectItem key={t} value={t}>
                  {NET_TYPE_LABEL[t]}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
        <div className='grid gap-1'>
          <Label htmlFor='net-protocol'>Protocol</Label>
          {/* Protocol follows the transport (Modbus on a bus, LoRaWAN on radio),
              so it's shown read-only rather than chosen independently. */}
          <Input id='net-protocol' value={value.protocol} readOnly disabled />
        </div>
        <div className='grid gap-1'>
          <Label htmlFor='net-max'>Max devices / network</Label>
          <Input
            id='net-max'
            type='number'
            min={1}
            value={value.maxDevices}
            onChange={(e) =>
              onChange({ ...value, maxDevices: Number(e.target.value) })
            }
          />
        </div>
      </div>

      {value.netType === '485' ? (
        <Params485
          value={value.p485}
          onChange={(p485) => onChange({ ...value, p485 })}
        />
      ) : value.netType === 'lora' ? (
        <ParamsLora
          value={value.pLora}
          onChange={(pLora) => onChange({ ...value, pLora })}
        />
      ) : (
        <ParamsEthernet
          value={value.pEth}
          onChange={(pEth) => onChange({ ...value, pEth })}
        />
      )}

      <div className='rounded-md border p-3'>
        <p className='text-muted-foreground mb-1 text-xs'>
          Will generate {keys.length} networks:
        </p>
        <p className='font-mono text-xs'>
          {keys.slice(0, 4).join(', ')}
          {keys.length > 4 ? `, … , ${keys[keys.length - 1]}` : ''}
        </p>
      </div>
    </div>
  )
}

export function networksStepValid(v: NetworksStepValue): boolean {
  return (
    Number.isFinite(v.count) &&
    v.count >= 1 &&
    v.count <= 200 &&
    v.namePattern.includes('{n}') &&
    Number.isFinite(v.maxDevices) &&
    v.maxDevices > 0
  )
}
