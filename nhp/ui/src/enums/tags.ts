/**
 * The TS mirror of `nhp/seed/tags.mjs` — NHP's single tag vocabulary, shared by
 * the wizards (WS-06) that PRODUCE records and the dashboard auto-build (WS-07)
 * that CONSUMES them. A tag drift here silently breaks the auto-built dashboards
 * (DASHBOARDS.md: "same tags ⇒ same page"), so the strings live in ONE place.
 *
 * NO DUPLICATION WITH DRIFT. `nhp/seed/tags.mjs` is the Node source of truth
 * (imported by the seed). It lives outside `src` and is untyped, so it cannot
 * enter the tsc build directly — this file is the typed UI mirror, and
 * `tags.unit.test.ts` imports BOTH and asserts every tag function produces an
 * identical result, so any drift fails the test gate. Edit `tags.mjs`, mirror here.
 *
 * Tags are carried in each record's `content.tags` (tags.mjs explains why: rubix
 * exposes no HTTP tag-attach route, so NHP off-loads to content). Conventions are
 * fixed by DOMAIN-MODEL.md §Tagging:
 *   tenant:<key> site:<key> gateway:<key> network:<key> meter:<key>
 *   group:<chart_group>   quantity:<q>   meter-type:<key>
 */
export const tenantTag = (key: string) => `tenant:${key}`
export const siteTag = (key: string) => `site:${key}`
export const gatewayTag = (key: string) => `gateway:${key}`
export const networkTag = (key: string) => `network:${key}`
export const meterTag = (key: string) => `meter:${key}`
export const groupTag = (group: string) => `group:${group}`
export const quantityTag = (quantity: string) => `quantity:${quantity}`
export const meterTypeTag = (key: string) => `meter-type:${key}`

/**
 * The hierarchy-membership tags a record at each level carries: its own level's
 * tag plus every ancestor's, so the dashboard auto-build can walk up from any
 * record (a meter page needs site:/gateway: context, etc).
 */
export function siteTags({ tenant }: { tenant: string }): string[] {
  return [tenantTag(tenant)]
}

export function gatewayTags({
  tenant,
  site,
}: {
  tenant: string
  site: string
}): string[] {
  return [tenantTag(tenant), siteTag(site)]
}

export function networkTags({
  tenant,
  site,
  gateway,
}: {
  tenant: string
  site: string
  gateway: string
}): string[] {
  return [tenantTag(tenant), siteTag(site), gatewayTag(gateway)]
}

export interface MeterTagCtx {
  tenant: string
  site: string
  gateway: string
  network: string
  meterType: string
}

export function meterTags({
  tenant,
  site,
  gateway,
  network,
  meterType,
}: MeterTagCtx): string[] {
  return [
    tenantTag(tenant),
    siteTag(site),
    gatewayTag(gateway),
    networkTag(network),
    meterTypeTag(meterType),
  ]
}

export interface RegisterTagCtx {
  tenant: string
  site: string
  gateway: string
  network: string
  meter: string
}

/**
 * A register inherits its meter's hierarchy tags plus its own grouping/quantity
 * tags — `group:<chart_group>` and `quantity:<q>`. DASHBOARDS.md §"Chart grouping".
 */
export function registerTags(
  { tenant, site, gateway, network, meter }: RegisterTagCtx,
  register: { chart_group?: string; quantity?: string }
): string[] {
  const tags = [
    tenantTag(tenant),
    siteTag(site),
    gatewayTag(gateway),
    networkTag(network),
    meterTag(meter),
  ]
  if (register.chart_group) tags.push(groupTag(register.chart_group))
  if (register.quantity) tags.push(quantityTag(register.quantity))
  return tags
}
