// The `meter-type` collection — an admin-defined template a meter is stamped from
// (DOMAIN-MODEL.md §meter-type). `version` bumps on every edit (versioning).
//
// `registers` is the template register set: an array of register-def objects
// (the same shape as the `register` collection below). It is free-form JSON;
// rubix has no `json` FieldType and allows undeclared content fields, so
// `registers` is left out of the declared schema and passes through as raw JSON
// content — the POC behaviour we want.

export const meterType = {
  kind: 'collection',
  name: 'meter-type',
  schema: [
    { name: 'key', type: 'text', required: true, unique: true },
    { name: 'name', type: 'text', required: true },
    { name: 'manufacturer', type: 'text' },
    { name: 'version', type: 'number', required: true },
    // `registers` is a free-form JSON array of register-defs — see module doc.
  ],
};
