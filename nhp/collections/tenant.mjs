// The `tenant` collection — a customer / organisation (DOMAIN-MODEL.md §tenant).
//
// A tenant maps to a rubix namespace for isolation; onboarding uses the rubix
// /tenants surface. Here we only define the record shape NHP records carry.

export const tenant = {
  kind: 'collection',
  name: 'tenant',
  schema: [
    { name: 'key', type: 'text', required: true, unique: true },
    { name: 'name', type: 'text', required: true },
    { name: 'namespace', type: 'text' },
  ],
};
