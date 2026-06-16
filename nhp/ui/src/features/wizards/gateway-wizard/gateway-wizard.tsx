/**
 * The gateway + N-networks wizard (WS-06 task 2). Threads two input steps
 * (gateway fields + parent site, then the bulk-networks generator) into the shared
 * WizardShell, which previews the 1 gateway + N networks and writes them
 * resumably. Verified to plan correctly at N=30 (plan.unit.test.ts).
 *
 * Loads the sites + tenants it needs to resolve the parent-site relation and the
 * tenant/site KEYS for the standard tags.
 */
import { useMemo, useState } from 'react'
import { WizardShell } from '../_shared/stepper'
import { useSites, useTenants } from '../_shared/hooks'
import {
  GatewayStep,
  gatewayStepValid,
  type GatewayStepValue,
} from './gateway-step'
import {
  NetworksStep,
  defaultNetworksStep,
  networksParams,
  networksStepValid,
  type NetworksStepValue,
} from './networks-step'
import { buildGatewayPlan } from './plan'

const EMPTY_GATEWAY: GatewayStepValue = {
  key: '',
  name: '',
  model: '',
  host: '',
  siteId: '',
}

export function GatewayWizard() {
  const sites = useSites()
  const tenants = useTenants()
  const [gw, setGw] = useState<GatewayStepValue>(EMPTY_GATEWAY)
  const [net, setNet] = useState<NetworksStepValue>(() =>
    defaultNetworksStep('')
  )

  const siteList = sites.data ?? []
  const tenantList = tenants.data ?? []

  // Resolve the selected site's key + its tenant's key for the tag context.
  const { siteKey, tenantKey } = useMemo(() => {
    const site = siteList.find((s) => s.id === gw.siteId)
    if (!site) return { siteKey: '', tenantKey: '' }
    const tenant = tenantList.find((t) => t.id === site.content.tenant)
    return { siteKey: site.content.key, tenantKey: tenant?.content.key ?? '' }
  }, [gw.siteId, siteList, tenantList])

  const buildPlan = () =>
    buildGatewayPlan(
      {
        key: gw.key,
        name: gw.name,
        model: gw.model,
        host: gw.host,
        siteId: gw.siteId,
        siteKey,
        tenantKey,
      },
      {
        count: net.count,
        netType: net.netType,
        protocol: net.protocol,
        maxDevices: net.maxDevices,
        namePattern: net.namePattern,
        params: networksParams(net),
      }
    )

  return (
    <WizardShell
      title='New gateway with N networks'
      description='Create a gateway and generate many networks on it in one batch — the "30 networks" flow.'
      steps={[
        {
          title: 'Gateway',
          valid: gatewayStepValid(gw),
          render: () => (
            <GatewayStep
              value={gw}
              onChange={(next) => {
                setGw(next)
                // Keep the default naming pattern tracking the gateway key until edited.
                if (net.namePattern === defaultNetworksStep(gw.key).namePattern) {
                  setNet((n) => ({
                    ...n,
                    namePattern: `${next.key || 'gw-01'}-net-{n}`,
                  }))
                }
              }}
              sites={siteList}
              tenants={tenantList}
            />
          ),
        },
        {
          title: 'Networks',
          valid: networksStepValid(net),
          render: () => <NetworksStep value={net} onChange={setNet} />,
        },
      ]}
      buildPlan={buildPlan}
    />
  )
}
