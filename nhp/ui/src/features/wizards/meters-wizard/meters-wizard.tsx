/**
 * Bulk-meters wizard (WS-06 task 3): pick a network (showing remaining capacity =
 * max_devices − current, via capacity.ts), pick a meter-type, give an inclusive
 * bus-address range + key/name patterns. The wizard BLOCKS an over-cap add (the
 * requested count cannot exceed the network's remaining capacity) — the cap-block
 * the brief calls for, reusing WS-05's capacity primitive. Each meter is stamped
 * from the type (registers + tags) by buildMetersPlan.
 */
import { useMemo, useState } from 'react'
import { capacityFor } from '@/features/gateways/capacity'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { WizardShell } from '../_shared/stepper'
import { networkAncestry } from '../_shared/ancestry'
import {
  useGateways,
  useMeterTypes,
  useMeters,
  useNetworks,
  useSites,
  useTenants,
} from '../_shared/hooks'
import { addressRange, buildMetersPlan } from './plan'

export function MetersWizard() {
  const networks = useNetworks()
  const gateways = useGateways()
  const sites = useSites()
  const tenants = useTenants()
  const meterTypes = useMeterTypes()
  const meters = useMeters()

  const [networkId, setNetworkId] = useState('')
  const [typeId, setTypeId] = useState('')
  const [from, setFrom] = useState(1)
  const [to, setTo] = useState(10)
  const [keyPattern, setKeyPattern] = useState('m-{n}')
  const [namePattern, setNamePattern] = useState('Meter {n}')

  const networkList = networks.data ?? []
  const typeList = meterTypes.data ?? []
  const meterList = meters.data ?? []

  const network = networkList.find((n) => n.id === networkId)
  const type = typeList.find((t) => t.id === typeId)

  const capacity = useMemo(
    () => (network ? capacityFor(network, meterList) : null),
    [network, meterList]
  )

  const requested = addressRange(from, to).length
  // THE CAP-BLOCK: requested count cannot exceed the network's remaining capacity.
  const overCap = capacity ? requested > capacity.remaining : false

  const buildPlan = () => {
    if (!network || !type) return []
    const anc = networkAncestry(
      network,
      gateways.data ?? [],
      sites.data ?? [],
      tenants.data ?? []
    )
    return buildMetersPlan({
      networkId: network.id,
      networkKey: anc.networkKey,
      tenantKey: anc.tenantKey,
      siteKey: anc.siteKey,
      gatewayKey: anc.gatewayKey,
      type,
      addressFrom: from,
      addressTo: to,
      keyPattern,
      namePattern,
    })
  }

  return (
    <WizardShell
      title='Bulk meters onto a network'
      description='Add a range of meters, stamped from a meter-type. Blocks anything over the device cap.'
      steps={[
        {
          title: 'Network & type',
          valid: networkId !== '' && typeId !== '',
          render: () => (
            <div className='grid gap-4'>
              <div className='grid gap-1'>
                <Label htmlFor='m-net'>Network</Label>
                <Select value={networkId} onValueChange={setNetworkId}>
                  <SelectTrigger id='m-net'>
                    <SelectValue placeholder='Select the parent network' />
                  </SelectTrigger>
                  <SelectContent>
                    {networkList.map((n) => {
                      const c = capacityFor(n, meterList)
                      return (
                        <SelectItem key={n.id} value={n.id}>
                          {n.content.key} — {c.remaining} of {c.cap} free
                        </SelectItem>
                      )
                    })}
                  </SelectContent>
                </Select>
                {capacity ? (
                  <p className='text-muted-foreground text-xs'>
                    {capacity.used}/{capacity.cap} used —{' '}
                    {capacity.remaining} remaining on this network.
                  </p>
                ) : null}
              </div>
              <div className='grid gap-1'>
                <Label htmlFor='m-type'>Meter-type</Label>
                <Select value={typeId} onValueChange={setTypeId}>
                  <SelectTrigger id='m-type'>
                    <SelectValue placeholder='Stamp meters from this type' />
                  </SelectTrigger>
                  <SelectContent>
                    {typeList.map((t) => (
                      <SelectItem key={t.id} value={t.id}>
                        {t.content.name} ({t.content.registers.length} registers)
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
            </div>
          ),
        },
        {
          title: 'Address range',
          valid: requested >= 1 && !overCap,
          render: () => (
            <div className='grid gap-4'>
              <div className='grid gap-4 sm:grid-cols-2'>
                <div className='grid gap-1'>
                  <Label htmlFor='m-from'>Address from</Label>
                  <Input
                    id='m-from'
                    type='number'
                    min={1}
                    value={from}
                    onChange={(e) => setFrom(Number(e.target.value))}
                  />
                </div>
                <div className='grid gap-1'>
                  <Label htmlFor='m-to'>Address to</Label>
                  <Input
                    id='m-to'
                    type='number'
                    min={1}
                    value={to}
                    onChange={(e) => setTo(Number(e.target.value))}
                  />
                </div>
              </div>
              <div className='grid gap-4 sm:grid-cols-2'>
                <div className='grid gap-1'>
                  <Label htmlFor='m-key'>Key pattern ({'{n}'} = address)</Label>
                  <Input
                    id='m-key'
                    value={keyPattern}
                    onChange={(e) => setKeyPattern(e.target.value)}
                  />
                </div>
                <div className='grid gap-1'>
                  <Label htmlFor='m-name'>Name pattern</Label>
                  <Input
                    id='m-name'
                    value={namePattern}
                    onChange={(e) => setNamePattern(e.target.value)}
                  />
                </div>
              </div>
              <p className='text-sm'>
                {requested} meters requested.
                {capacity ? ` ${capacity.remaining} slots free.` : ''}
              </p>
              {overCap ? (
                <p className='text-destructive text-sm'>
                  Over capacity — {requested} meters exceed the{' '}
                  {capacity?.remaining} remaining. Lower the range or raise the
                  network's max_devices.
                </p>
              ) : null}
            </div>
          ),
        },
      ]}
      buildPlan={buildPlan}
    />
  )
}
