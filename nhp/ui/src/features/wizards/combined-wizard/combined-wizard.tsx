/**
 * The combined "add everything" wizard (WS-06 task 5): chains tenant → site →
 * gateway(+N networks) → meters into ONE plan (buildCombinedPlan) with threaded
 * parent ids, shown in a single final preview of the whole tree before writing.
 * Reuses the bulk-networks step (gateway-wizard) and meter-type/range inputs;
 * tags come from the shared module so the new tree auto-builds dashboards. The
 * first user is optional and created separately on the principal API (admin),
 * outside the records batch.
 */
import { useState } from 'react'
import type { NetType } from '@/enums/options'
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
import { useMeterTypes } from '../_shared/hooks'
import {
  NetworksStep,
  defaultNetworksStep,
  networksParams,
  networksStepValid,
  type NetworksStepValue,
} from '../gateway-wizard/networks-step'
import { addressRange } from '../meters-wizard/plan'
import { buildCombinedPlan } from './plan'

export function CombinedWizard() {
  const meterTypes = useMeterTypes()
  const typeList = meterTypes.data ?? []

  const [tenant, setTenant] = useState({ key: '', name: '' })
  const [site, setSite] = useState({
    key: '',
    name: '',
    address: '',
    timezone: 'UTC',
  })
  const [gateway, setGateway] = useState({
    key: '',
    name: '',
    model: '',
    host: '',
  })
  const [net, setNet] = useState<NetworksStepValue>(() =>
    defaultNetworksStep('')
  )
  const [typeId, setTypeId] = useState('')
  const [from, setFrom] = useState(1)
  const [to, setTo] = useState(4)

  const type = typeList.find((t) => t.id === typeId)
  const requested = addressRange(from, to).length
  // Meters land on the first network — block a range over its max_devices.
  const metersOverCap = type ? requested > net.maxDevices : false

  const buildPlan = () =>
    buildCombinedPlan({
      tenant,
      site,
      gateway,
      networks: {
        count: net.count,
        netType: net.netType as NetType,
        protocol: net.protocol,
        maxDevices: net.maxDevices,
        namePattern: net.namePattern,
        params: networksParams(net),
      },
      meters: {
        type,
        addressFrom: from,
        addressTo: to,
        keyPattern: `${gateway.key || 'gw-01'}-net-1-m{n}`,
        namePattern: 'Meter {n}',
      },
    })

  return (
    <WizardShell
      title='Add everything'
      description='Greenfield onboarding: tenant → site → gateway(+networks) → meters in one previewed batch.'
      steps={[
        {
          title: 'Tenant',
          valid: tenant.key.trim() !== '' && tenant.name.trim() !== '',
          render: () => (
            <div className='grid gap-4 sm:grid-cols-2'>
              <Field
                id='c-tkey'
                label='Tenant key'
                value={tenant.key}
                onChange={(key) => setTenant({ ...tenant, key })}
                placeholder='acme'
              />
              <Field
                id='c-tname'
                label='Tenant name'
                value={tenant.name}
                onChange={(name) => setTenant({ ...tenant, name })}
                placeholder='Acme Corp'
              />
            </div>
          ),
        },
        {
          title: 'Site',
          valid: site.key.trim() !== '' && site.name.trim() !== '',
          render: () => (
            <div className='grid gap-4 sm:grid-cols-2'>
              <Field
                id='c-skey'
                label='Site key'
                value={site.key}
                onChange={(key) => setSite({ ...site, key })}
                placeholder='hq'
              />
              <Field
                id='c-sname'
                label='Site name'
                value={site.name}
                onChange={(name) => setSite({ ...site, name })}
                placeholder='Head office'
              />
              <Field
                id='c-saddr'
                label='Address'
                value={site.address}
                onChange={(address) => setSite({ ...site, address })}
              />
              <Field
                id='c-stz'
                label='Timezone'
                value={site.timezone}
                onChange={(timezone) => setSite({ ...site, timezone })}
                placeholder='Australia/Sydney'
              />
            </div>
          ),
        },
        {
          title: 'Gateway',
          valid: gateway.key.trim() !== '' && gateway.name.trim() !== '',
          render: () => (
            <div className='grid gap-4 sm:grid-cols-2'>
              <Field
                id='c-gkey'
                label='Gateway key'
                value={gateway.key}
                onChange={(key) => {
                  setGateway({ ...gateway, key })
                  if (
                    net.namePattern ===
                    defaultNetworksStep(gateway.key).namePattern
                  ) {
                    setNet((n) => ({
                      ...n,
                      namePattern: `${key || 'gw-01'}-net-{n}`,
                    }))
                  }
                }}
                placeholder='gw-01'
              />
              <Field
                id='c-gname'
                label='Gateway name'
                value={gateway.name}
                onChange={(name) => setGateway({ ...gateway, name })}
              />
              <Field
                id='c-gmodel'
                label='Model'
                value={gateway.model}
                onChange={(model) => setGateway({ ...gateway, model })}
              />
              <Field
                id='c-ghost'
                label='Host'
                value={gateway.host}
                onChange={(host) => setGateway({ ...gateway, host })}
              />
            </div>
          ),
        },
        {
          title: 'Networks',
          valid: networksStepValid(net),
          render: () => <NetworksStep value={net} onChange={setNet} />,
        },
        {
          title: 'Meters (optional)',
          valid: !metersOverCap,
          render: () => (
            <div className='grid gap-4'>
              <div className='grid gap-1'>
                <Label htmlFor='c-mtype'>Meter-type (optional)</Label>
                <Select value={typeId} onValueChange={setTypeId}>
                  <SelectTrigger id='c-mtype'>
                    <SelectValue placeholder='None — skip meters' />
                  </SelectTrigger>
                  <SelectContent>
                    {typeList.map((t) => (
                      <SelectItem key={t.id} value={t.id}>
                        {t.content.name}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
                <p className='text-muted-foreground text-xs'>
                  Meters land on the first network ({net.namePattern.replace(
                    '{n}',
                    '1'
                  )}).
                </p>
              </div>
              {type ? (
                <>
                  <div className='grid gap-4 sm:grid-cols-2'>
                    <div className='grid gap-1'>
                      <Label htmlFor='c-mfrom'>Address from</Label>
                      <Input
                        id='c-mfrom'
                        type='number'
                        min={1}
                        value={from}
                        onChange={(e) => setFrom(Number(e.target.value))}
                      />
                    </div>
                    <div className='grid gap-1'>
                      <Label htmlFor='c-mto'>Address to</Label>
                      <Input
                        id='c-mto'
                        type='number'
                        min={1}
                        value={to}
                        onChange={(e) => setTo(Number(e.target.value))}
                      />
                    </div>
                  </div>
                  <p className='text-sm'>
                    {requested} meters on the first network (cap{' '}
                    {net.maxDevices}).
                  </p>
                  {metersOverCap ? (
                    <p className='text-destructive text-sm'>
                      Over capacity — {requested} exceeds the {net.maxDevices}{' '}
                      device cap. Lower the range or raise max_devices.
                    </p>
                  ) : null}
                </>
              ) : null}
            </div>
          ),
        },
      ]}
      buildPlan={buildPlan}
    />
  )
}

/** Small labelled text input — local to this wizard's compact multi-field steps. */
function Field({
  id,
  label,
  value,
  onChange,
  placeholder,
}: {
  id: string
  label: string
  value: string
  onChange: (v: string) => void
  placeholder?: string
}) {
  return (
    <div className='grid gap-1'>
      <Label htmlFor={id}>{label}</Label>
      <Input
        id={id}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder={placeholder}
      />
    </div>
  )
}
