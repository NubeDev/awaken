// The single source of truth for NHP's standard tags — imported by the seed AND
// by the dashboard auto-build (WS-07) and wizards (WS-06) so every producer and
// consumer agrees on the exact strings. A tag drift here silently breaks the
// auto-built dashboards (DASHBOARDS.md: "same tags ⇒ same page"), so it lives in
// ONE module.
//
// How tags are stored (POC decision). rubix tags are graph edges
// (record→tagged→tag), but the only way to attach them is the gate library — the
// HTTP records API exposes NO tag-attach route (verified: no tag write in
// rubix/crates/rubix-server/src/http/records/). The list read projects a record's
// tags onto `RecordDto.tags`, but single reads return none. Since NHP writes
// everything over HTTP (OVERVIEW: NHP is UI + DATA on the unchanged rubix binary),
// the standard tags are carried in the record's `content.tags` array. WS-07 reads
// `content.tags`. This keeps NHP off any rubix change.
//
// Tag conventions are fixed by DOMAIN-MODEL.md §Tagging + DASHBOARDS.md:
//   tenant:<key> site:<key> gateway:<key> network:<key> meter:<key>
//   group:<chart_group>   quantity:<q>   meter-type:<key>

export const tenantTag = (key) => `tenant:${key}`;
export const siteTag = (key) => `site:${key}`;
export const gatewayTag = (key) => `gateway:${key}`;
export const networkTag = (key) => `network:${key}`;
export const meterTag = (key) => `meter:${key}`;
export const groupTag = (group) => `group:${group}`;
export const quantityTag = (quantity) => `quantity:${quantity}`;
export const meterTypeTag = (key) => `meter-type:${key}`;

// The hierarchy-membership tags a record at a given level carries. Each level
// carries its OWN tag plus every ancestor's, so the dashboard auto-build can walk
// up from any record (a meter page needs site:/gateway: context, etc).

export function siteTags({ tenant }) {
  return [tenantTag(tenant)];
}

export function gatewayTags({ tenant, site }) {
  return [tenantTag(tenant), siteTag(site)];
}

export function networkTags({ tenant, site, gateway }) {
  return [tenantTag(tenant), siteTag(site), gatewayTag(gateway)];
}

export function meterTags({ tenant, site, gateway, network, meterType }) {
  return [
    tenantTag(tenant),
    siteTag(site),
    gatewayTag(gateway),
    networkTag(network),
    meterTypeTag(meterType),
  ];
}

// A register inherits its meter's hierarchy tags plus its own grouping/quantity
// tags — `group:<chart_group>` (one multi-series chart per group) and
// `quantity:<q>` (cross-meter rollup). DASHBOARDS.md §"Chart grouping".
export function registerTags({ tenant, site, gateway, network, meter }, register) {
  const tags = [
    tenantTag(tenant),
    siteTag(site),
    gatewayTag(gateway),
    networkTag(network),
    meterTag(meter),
  ];
  if (register.chart_group) tags.push(groupTag(register.chart_group));
  if (register.quantity) tags.push(quantityTag(register.quantity));
  return tags;
}
