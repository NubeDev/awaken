// The `network` collection — a comms bus on a gateway (DOMAIN-MODEL.md §network).
// `gateway` is the parent relation. `net_type` (485/ethernet) and `protocol`
// (modbus) are closed enums (nhp/collections/enums.mjs) modelled as text. The
// per-network device cap is `max_devices`; its enforcement is the meter
// device-limit check (see nhp/collections/register-collections.mjs and
// DOMAIN-MODEL "Device limit") — rubix's gate cannot express a count writeRule.
//
// `params` (net-type-specific baud/parity or ip/port) is free-form JSON. rubix
// has no `json` FieldType and its validate step ALLOWS undeclared content fields,
// so `params` is intentionally left out of the declared schema: it passes through
// as raw JSON content, unvalidated, which is the POC behaviour we want.

export const network = {
  kind: 'collection',
  name: 'network',
  schema: [
    { name: 'key', type: 'text', required: true, unique: true },
    { name: 'name', type: 'text' },
    { name: 'gateway', type: 'relation', required: true },
    { name: 'net_type', type: 'text' },
    { name: 'protocol', type: 'text' },
    { name: 'max_devices', type: 'number', required: true },
    // `params` is free-form JSON — see module doc; not declared.
  ],
};
