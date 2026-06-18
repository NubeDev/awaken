/**
 * Create / edit a `kind:"network"` under a gateway (ADMIN.md §5, DOMAIN-MODEL
 * §network). `net_type` (485 | ethernet) switches the `params` sub-form
 * (params-485 vs params-ethernet); `protocol` and `max_devices` are common.
 *
 * Device limit (DOMAIN-MODEL "Device limit"): `max_devices` is the per-network
 * cap. On EDIT the form blocks lowering the cap below the meters already on the
 * network (capacity.ts) — the same client-side defence the add-meter UI uses,
 * since the rubix gate can't count (WS-02 finding, logged RUBIX-TEAM).
 */
import { useState } from 'react'
import type {
  GatewayRecord,
  MeterRecord,
  Net485Params,
  NetEthernetParams,
  NetLoraParams,
  NetParams,
  Network,
  NetworkRecord,
} from '@/api/records'
import {
  NET_TYPE,
  PROTOCOL,
  toOptions,
  type NetType,
  type Protocol,
} from '@/enums/options'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { metersOnNetwork } from './capacity'
import { useCreateNetwork, useUpdateNetwork } from './hooks'
import { default485, Params485 } from './params-485'
import { defaultEthernet, ParamsEthernet } from './params-ethernet'
import { defaultLora, ParamsLora } from './params-lora'

type NetworkFormProps = {
  open: boolean
  onOpenChange: (open: boolean) => void
  gateway: GatewayRecord
  /** Present = edit; absent = create a new network on `gateway`. */
  network?: NetworkRecord
  /** All meters — to block lowering a cap below current usage. */
  meters: MeterRecord[]
}

const NET_TYPE_LABEL: Record<NetType, string> = {
  '485': 'RS-485 (serial)',
  ethernet: 'Ethernet (TCP)',
  lora: 'LoRaWAN (radio)',
}

// The protocol a transport carries: a LoRa radio speaks LoRaWAN, the field buses
// speak Modbus. Switching net_type re-pins protocol so the two never disagree.
const PROTOCOL_FOR: Record<NetType, Protocol> = {
  '485': 'modbus',
  ethernet: 'modbus',
  lora: 'lora',
}

export function NetworkForm({
  open,
  onOpenChange,
  gateway,
  network,
  meters,
}: NetworkFormProps) {
  const editing = network !== undefined
  const start = network?.content
  const [key, setKey] = useState(start?.key ?? '')
  const [name, setName] = useState(start?.name ?? '')
  const [netType, setNetType] = useState<NetType>(start?.net_type ?? '485')
  const [protocol, setProtocol] = useState(start?.protocol ?? PROTOCOL[0])
  const [maxDevices, setMaxDevices] = useState(start?.max_devices ?? 16)
  // Keep both param shapes around so toggling net_type doesn't lose typed values.
  const [p485, setP485] = useState<Net485Params>(
    start?.net_type === '485' ? (start.params as Net485Params) : default485()
  )
  const [pEth, setPEth] = useState<NetEthernetParams>(
    start?.net_type === 'ethernet'
      ? (start.params as NetEthernetParams)
      : defaultEthernet()
  )
  const [pLora, setPLora] = useState<NetLoraParams>(
    start?.net_type === 'lora' ? (start.params as NetLoraParams) : defaultLora()
  )

  // Switching transport re-pins the protocol (a LoRa bus can't speak Modbus).
  const onNetType = (v: NetType) => {
    setNetType(v)
    setProtocol(PROTOCOL_FOR[v])
  }

  const create = useCreateNetwork()
  const update = useUpdateNetwork()
  const pending = create.isPending || update.isPending

  // On edit, the network's record id is what meters point their `network` at.
  const usedOnThis = network
    ? metersOnNetwork(network.id, meters).length
    : 0
  const capTooLow = editing && maxDevices < usedOnThis
  const valid =
    key.trim() !== '' && Number.isFinite(maxDevices) && maxDevices > 0 && !capTooLow

  const save = () => {
    const params: NetParams =
      netType === '485' ? p485 : netType === 'lora' ? pLora : pEth
    const base: Network = {
      kind: 'network',
      key,
      name,
      gateway: gateway.id, // relation by record id (matches WS-03 seed portfolio.mjs)
      net_type: netType,
      protocol,
      max_devices: maxDevices,
      params,
    }
    if (editing && network) {
      update.mutate(
        { id: network.id, content: { ...network.content, ...base } },
        { onSuccess: () => onOpenChange(false) }
      )
      return
    }
    create.mutate(base, { onSuccess: () => onOpenChange(false) })
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className='max-w-lg'>
        <DialogHeader>
          <DialogTitle>
            {editing ? 'Edit network' : 'New network'} on {gateway.content.name}
          </DialogTitle>
          <DialogDescription>
            A communications bus on the gateway. The type selects its serial / TCP
            settings; the device limit caps how many meters it carries.
          </DialogDescription>
        </DialogHeader>

        <div className='grid gap-4'>
          <div className='grid gap-4 sm:grid-cols-2'>
            <div className='grid gap-1'>
              <Label htmlFor='net-key'>Key</Label>
              <Input
                id='net-key'
                value={key}
                onChange={(e) => setKey(e.target.value)}
                placeholder='gw-01-net-1'
              />
            </div>
            <div className='grid gap-1'>
              <Label htmlFor='net-name'>Name</Label>
              <Input
                id='net-name'
                value={name}
                onChange={(e) => setName(e.target.value)}
                placeholder='Basement RS-485'
              />
            </div>
          </div>

          <div className='grid gap-4 sm:grid-cols-3'>
            <div className='grid gap-1'>
              <Label htmlFor='net-type'>Type</Label>
              <Select
                value={netType}
                onValueChange={(v) => onNetType(v as NetType)}
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
              <Select
                value={protocol}
                onValueChange={(v) => setProtocol(v as typeof protocol)}
              >
                <SelectTrigger id='net-protocol'>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {toOptions(PROTOCOL).map((o) => (
                    <SelectItem key={o.value} value={o.value}>
                      {o.label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
            <div className='grid gap-1'>
              <Label htmlFor='net-max'>Max devices</Label>
              <Input
                id='net-max'
                type='number'
                min={1}
                value={maxDevices}
                onChange={(e) => setMaxDevices(Number(e.target.value))}
              />
            </div>
          </div>

          {/* net_type switches the params sub-form (DOMAIN-MODEL §network). */}
          {netType === '485' ? (
            <Params485 value={p485} onChange={setP485} />
          ) : netType === 'lora' ? (
            <ParamsLora value={pLora} onChange={setPLora} />
          ) : (
            <ParamsEthernet value={pEth} onChange={setPEth} />
          )}

          {capTooLow ? (
            <p className='text-sm text-amber-600'>
              {usedOnThis} meters are on this network — the cap can't go below that.
            </p>
          ) : null}
        </div>

        <DialogFooter>
          <Button variant='ghost' onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button onClick={save} disabled={!valid || pending}>
            {pending ? 'Saving…' : 'Save'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
