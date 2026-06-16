// The `gateway` collection — a field device hosting network ports
// (DOMAIN-MODEL.md §gateway). `site` is the parent relation. `status` and
// `last_seen` are poller-owned (written by the polling service, read by NHP);
// `status` is a closed enum (see nhp/collections/enums.mjs STATUS) modelled as
// text — rubix has no native enum and its gate does not enforce an allowed-set.

export const gateway = {
  kind: 'collection',
  name: 'gateway',
  schema: [
    { name: 'key', type: 'text', required: true, unique: true },
    { name: 'name', type: 'text', required: true },
    { name: 'site', type: 'relation', required: true },
    { name: 'model', type: 'text' },
    { name: 'host', type: 'text' },
    { name: 'status', type: 'text' },
    { name: 'last_seen', type: 'date' },
  ],
};
