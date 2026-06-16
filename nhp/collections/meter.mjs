// The `meter` collection — a metering device on a network (DOMAIN-MODEL.md
// §meter). `network` and `meter_type` are relations; a meter is stamped from a
// meter-type at creation and records `meter_type_version` (the type version at
// stamp time — versioning, DOMAIN-MODEL §versioning; the stamping logic lands in
// WS-04). `status`/`last_seen` are poller-owned. `address` is the bus unit id.
//
// The per-network device limit is enforced against the parent network's
// `max_devices` by the registrar/UI, not the gate (rubix evaluates no writeRule);
// see nhp/collections/register-collections.mjs.

export const meter = {
  kind: 'collection',
  name: 'meter',
  schema: [
    { name: 'key', type: 'text', required: true, unique: true },
    { name: 'name', type: 'text', required: true },
    { name: 'network', type: 'relation', required: true },
    { name: 'meter_type', type: 'relation', required: true },
    { name: 'meter_type_version', type: 'number' },
    { name: 'address', type: 'number', required: true },
    { name: 'status', type: 'text' },
    { name: 'last_seen', type: 'date' },
  ],
};
