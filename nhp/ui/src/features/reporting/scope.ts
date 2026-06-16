/**
 * Portfolio scope resolution for the reporting + alarms surfaces. Pure functions
 * over the record arrays (no React, no fetching) so they are trivially testable
 * (scope.unit.test.ts) and shared by every report type and the alarm console.
 *
 * The domain is a hierarchy — tenant → site → gateway → network → meter →
 * register — and relations are stored child→parent by record id (WS-03 seed). A
 * meter therefore only KNOWS its network; its site and tenant are reached by
 * walking network → gateway → site → tenant. `buildIndex` does that walk once and
 * memoises it into lookup maps; `selectMeters` / `selectRegisters` then filter by
 * a `ScopeFilter` (tenant, site, meter-type, quantity) for the whole surface.
 */
import type {
  GatewayRecord,
  MeterRecord,
  MeterTypeRecord,
  NetworkRecord,
  RegisterRec,
  SiteRecord,
  TenantRecord,
} from '@/api/records'
import { hasAlarm } from '@/features/dashboards/_shared/field-config'

export interface PortfolioData {
  tenants: TenantRecord[]
  sites: SiteRecord[]
  gateways: GatewayRecord[]
  networks: NetworkRecord[]
  meters: MeterRecord[]
  registers: RegisterRec[]
  meterTypes: MeterTypeRecord[]
}

/** Where a meter sits in the hierarchy, with display labels resolved. */
export interface MeterLocation {
  meterId: string
  siteId?: string
  siteName: string
  tenantId?: string
  tenantName: string
}

export interface PortfolioIndex {
  data: PortfolioData
  meterById: Map<string, MeterRecord>
  /** meterId → its resolved site/tenant (via network → gateway → site → tenant). */
  meterLocation: Map<string, MeterLocation>
  /** meterId → its registers. */
  registersByMeter: Map<string, RegisterRec[]>
  meterTypeName: (id?: string) => string
}

export interface ScopeFilter {
  tenantId?: string
  siteId?: string
  meterTypeId?: string
  /** Register `quantity` (e.g. `power`, `energy`, `voltage`). */
  quantity?: string
}

export function buildIndex(data: PortfolioData): PortfolioIndex {
  const netById = new Map(data.networks.map((n) => [n.id, n.content]))
  const gwById = new Map(data.gateways.map((g) => [g.id, g.content]))
  const siteById = new Map(data.sites.map((s) => [s.id, s.content]))
  const tenantById = new Map(data.tenants.map((t) => [t.id, t.content]))
  const typeById = new Map(data.meterTypes.map((t) => [t.id, t.content]))

  const meterById = new Map(data.meters.map((m) => [m.id, m]))

  const meterLocation = new Map<string, MeterLocation>()
  for (const m of data.meters) {
    const net = m.content.network ? netById.get(m.content.network) : undefined
    const gw = net?.gateway ? gwById.get(net.gateway) : undefined
    const siteId = gw?.site
    const site = siteId ? siteById.get(siteId) : undefined
    const tenantId = site?.tenant
    const tenant = tenantId ? tenantById.get(tenantId) : undefined
    meterLocation.set(m.id, {
      meterId: m.id,
      siteId,
      siteName: site?.name ?? '—',
      tenantId,
      tenantName: tenant?.name ?? '—',
    })
  }

  const registersByMeter = new Map<string, RegisterRec[]>()
  for (const r of data.registers) {
    const list = registersByMeter.get(r.content.meter) ?? []
    list.push(r)
    registersByMeter.set(r.content.meter, list)
  }

  return {
    data,
    meterById,
    meterLocation,
    registersByMeter,
    meterTypeName: (id?: string) => (id ? (typeById.get(id)?.name ?? '—') : '—'),
  }
}

/** Meters matching the tenant / site / meter-type parts of a filter. */
export function selectMeters(
  index: PortfolioIndex,
  filter: ScopeFilter
): MeterRecord[] {
  return index.data.meters.filter((m) => {
    const loc = index.meterLocation.get(m.id)
    if (filter.tenantId && loc?.tenantId !== filter.tenantId) return false
    if (filter.siteId && loc?.siteId !== filter.siteId) return false
    if (filter.meterTypeId && m.content.meter_type !== filter.meterTypeId)
      return false
    return true
  })
}

/**
 * History-bearing registers under a filter. Applies the meter scope (tenant /
 * site / meter-type) then the register-level `quantity` filter; `alarmsOnly`
 * keeps only registers that define an alarm ramp.
 */
export function selectRegisters(
  index: PortfolioIndex,
  filter: ScopeFilter,
  opts: { alarmsOnly?: boolean; includeNoHistory?: boolean } = {}
): RegisterRec[] {
  const meterIds = new Set(selectMeters(index, filter).map((m) => m.id))
  return index.data.registers.filter((r) => {
    if (!meterIds.has(r.content.meter)) return false
    if (!opts.includeNoHistory && r.content.history === false) return false
    if (filter.quantity && r.content.quantity !== filter.quantity) return false
    if (opts.alarmsOnly && !hasAlarm(r.content.alarm)) return false
    return true
  })
}

/** Distinct sites under a tenant filter (for the site picker). */
export function sitesForTenant(
  index: PortfolioIndex,
  tenantId?: string
): SiteRecord[] {
  return index.data.sites.filter(
    (s) => !tenantId || s.content.tenant === tenantId
  )
}

/** Distinct register quantities present in the meters a filter selects. */
export function quantitiesInScope(
  index: PortfolioIndex,
  filter: ScopeFilter
): string[] {
  const regs = selectRegisters(
    index,
    { ...filter, quantity: undefined },
    { includeNoHistory: true }
  )
  const set = new Set<string>()
  for (const r of regs) if (r.content.quantity) set.add(r.content.quantity)
  return [...set].sort()
}
