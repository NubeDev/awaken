/**
 * Resolve a record's hierarchy KEYS by walking the relation chain up the loaded
 * record lists. The wizards need the ancestor KEYS (not ids) because the standard
 * tags are key-based (`tenant:<key>` …, enums/tags.ts) — but records store the
 * parent's record ID in the relation field. This climbs network → gateway → site
 * → tenant, returning each level's `content.key`. Pure; reused by the meters and
 * combined wizards.
 */
import type {
  GatewayRecord,
  NetworkRecord,
  RecordDto,
  SiteRecord,
} from '@/api/records'

interface TenantLike {
  key: string
}

export interface NetworkAncestry {
  networkKey: string
  gatewayKey: string
  siteKey: string
  tenantKey: string
}

/** Walk up from a network record to its tenant key via the loaded lists. */
export function networkAncestry(
  network: NetworkRecord,
  gateways: GatewayRecord[],
  sites: SiteRecord[],
  tenants: RecordDto<TenantLike>[]
): NetworkAncestry {
  const gateway = gateways.find((g) => g.id === network.content.gateway)
  const site = sites.find((s) => s.id === gateway?.content.site)
  const tenant = tenants.find((t) => t.id === site?.content.tenant)
  return {
    networkKey: network.content.key,
    gatewayKey: gateway?.content.key ?? '',
    siteKey: site?.content.key ?? '',
    tenantKey: tenant?.content.key ?? '',
  }
}
