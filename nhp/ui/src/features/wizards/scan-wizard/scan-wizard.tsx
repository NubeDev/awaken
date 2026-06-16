/**
 * Scan-to-add-a-device wizard (WS-09 Part B). Flow: scan a barcode (camera, with a
 * manual-entry fallback) → resolve it to a meter-type (enums/barcode.ts) → pick the
 * parent network (showing remaining capacity via capacity.ts, BLOCKING over-cap) +
 * the meter's bus address + name → preview → write ONE meter, stamped from the type
 * (registers + the standard tags) via the shared batch writer. Honest error cases:
 * an unknown barcode (no matching meter-type) shows a clear error; an over-cap
 * network is blocked with the remaining count. See nhp/docs/WIZARDS.md.
 */
import { useMemo, useState } from 'react'
import { CheckCircle2 } from 'lucide-react'
import { capacityFor } from '@/features/gateways/capacity'
import { resolveBarcode } from '@/enums/barcode'
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
import { Scanner } from './scanner'
import { buildScanMeterPlan } from './plan'

export function ScanWizard() {
  const networks = useNetworks()
  const gateways = useGateways()
  const sites = useSites()
  const tenants = useTenants()
  const meterTypes = useMeterTypes()
  const meters = useMeters()

  const [scanned, setScanned] = useState('')
  const [networkId, setNetworkId] = useState('')
  const [address, setAddress] = useState(1)
  const [meterKey, setMeterKey] = useState('')
  const [meterName, setMeterName] = useState('')

  const typeList = meterTypes.data ?? []
  const networkList = networks.data ?? []
  const meterList = meters.data ?? []

  // Resolve the scanned/typed code to a meter-type. null when it doesn't decode or
  // no meter-type has that key — the wizard shows the unknown-barcode error.
  const resolved = useMemo(
    () => (scanned ? resolveBarcode(scanned, typeList) : null),
    [scanned, typeList]
  )
  const unknown = scanned !== '' && resolved === null

  const network = networkList.find((n) => n.id === networkId)
  const capacity = useMemo(
    () => (network ? capacityFor(network, meterList) : null),
    [network, meterList]
  )
  // THE CAP-BLOCK: one device can't be added to a network with no slots left.
  const overCap = capacity ? capacity.remaining < 1 : false

  const onCode = (code: string) => {
    setScanned(code)
    const t = resolveBarcode(code, typeList)
    // Seed sensible defaults for the new meter once the type is known.
    if (t && !meterKey) {
      setMeterKey(`${t.content.key}-{addr}`)
      setMeterName(`${t.content.name}`)
    }
  }

  const resolveKey = (raw: string) =>
    raw.replace(/\{addr\}/g, String(address)).trim()

  const buildPlan = () => {
    if (!resolved || !network) return []
    const anc = networkAncestry(
      network,
      gateways.data ?? [],
      sites.data ?? [],
      tenants.data ?? []
    )
    return buildScanMeterPlan({
      networkId: network.id,
      networkKey: anc.networkKey,
      tenantKey: anc.tenantKey,
      siteKey: anc.siteKey,
      gatewayKey: anc.gatewayKey,
      type: resolved,
      address,
      meterKey: resolveKey(meterKey),
      meterName: meterName.trim(),
    })
  }

  return (
    <WizardShell
      title='Scan to add a device'
      description='Scan a meter-type barcode to add one meter, stamped from that type. Camera or manual entry; blocks over-cap networks.'
      steps={[
        {
          title: 'Scan barcode',
          valid: resolved !== null,
          render: () => (
            <div className='grid gap-4'>
              <Scanner onCode={onCode} />
              {resolved ? (
                <div className='flex items-center gap-2 rounded-md border border-emerald-500/40 p-3 text-sm text-emerald-600'>
                  <CheckCircle2 className='size-4' />
                  Resolved to <strong>{resolved.content.name}</strong> (
                  {resolved.content.registers.length} registers, v
                  {resolved.content.version}).
                </div>
              ) : null}
              {unknown ? (
                <div className='text-destructive rounded-md border border-destructive/40 p-3 text-sm'>
                  Unknown barcode <code>{scanned}</code> — no meter-type matches
                  this code. Print the label from a meter-type, or check the code.
                </div>
              ) : null}
            </div>
          ),
        },
        {
          title: 'Network & meter',
          valid:
            networkId !== '' &&
            !overCap &&
            resolveKey(meterKey) !== '' &&
            meterName.trim() !== '' &&
            Number.isFinite(address) &&
            address >= 1,
          render: () => (
            <div className='grid gap-4'>
              <div className='grid gap-1'>
                <Label htmlFor='s-net'>Parent network</Label>
                <Select value={networkId} onValueChange={setNetworkId}>
                  <SelectTrigger id='s-net'>
                    <SelectValue placeholder='Select the parent network' />
                  </SelectTrigger>
                  <SelectContent>
                    {networkList.map((n) => {
                      const c = capacityFor(n, meterList)
                      return (
                        <SelectItem
                          key={n.id}
                          value={n.id}
                          disabled={c.remaining < 1}
                        >
                          {n.content.key} — {c.remaining} of {c.cap} free
                        </SelectItem>
                      )
                    })}
                  </SelectContent>
                </Select>
                {capacity ? (
                  <p className='text-muted-foreground text-xs'>
                    {capacity.used}/{capacity.cap} used — {capacity.remaining}{' '}
                    remaining on this network.
                  </p>
                ) : null}
                {overCap ? (
                  <p className='text-destructive text-sm'>
                    This network is full ({capacity?.used}/{capacity?.cap}). Pick
                    another or raise its max_devices.
                  </p>
                ) : null}
              </div>
              <div className='grid gap-4 sm:grid-cols-3'>
                <div className='grid gap-1'>
                  <Label htmlFor='s-addr'>Bus address</Label>
                  <Input
                    id='s-addr'
                    type='number'
                    min={1}
                    value={address}
                    onChange={(e) => setAddress(Number(e.target.value))}
                  />
                </div>
                <div className='grid gap-1'>
                  <Label htmlFor='s-key'>Key ({'{addr}'} = address)</Label>
                  <Input
                    id='s-key'
                    value={meterKey}
                    onChange={(e) => setMeterKey(e.target.value)}
                    className='font-mono text-sm'
                  />
                </div>
                <div className='grid gap-1'>
                  <Label htmlFor='s-name'>Name</Label>
                  <Input
                    id='s-name'
                    value={meterName}
                    onChange={(e) => setMeterName(e.target.value)}
                  />
                </div>
              </div>
              <p className='text-muted-foreground text-sm'>
                Stamps 1 meter + {resolved?.content.registers.length ?? 0}{' '}
                registers from <strong>{resolved?.content.name}</strong> as{' '}
                <code>{resolveKey(meterKey) || '—'}</code>.
              </p>
            </div>
          ),
        },
      ]}
      buildPlan={buildPlan}
    />
  )
}
