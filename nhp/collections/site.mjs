// The `site` collection — a physical location belonging to a tenant
// (DOMAIN-MODEL.md §site). `tenant` is the parent relation (the child stores the
// parent record id).

export const site = {
  kind: 'collection',
  name: 'site',
  schema: [
    { name: 'key', type: 'text', required: true, unique: true },
    { name: 'name', type: 'text', required: true },
    { name: 'tenant', type: 'relation', required: true },
    { name: 'address', type: 'text' },
    { name: 'timezone', type: 'text' },
    { name: 'geo', type: 'text' },
  ],
};
