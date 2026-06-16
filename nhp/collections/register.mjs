// The `register` collection — one readable/writable point on a meter: the Modbus
// metadata contract the polling service consumes plus the semantics/presentation
// NHP renders (DOMAIN-MODEL.md §register). The concrete record under a meter,
// stamped from a meter-type's register-def at meter creation (WS-04).
//
// Closed enums (`fn_code`, `datatype`, `byte_order`, `chart_type`) are text —
// allowed sets in nhp/collections/enums.mjs, enforced by registrar/UI, not the
// gate. `alarm` is a free-form JSON threshold object (DOMAIN-MODEL §alarms);
// rubix has no `json` FieldType and allows undeclared fields, so `alarm` is left
// out of the declared schema and passes through as raw JSON content.
//
// `meter` is the parent relation. A register also carries a `key` unique within
// its meter (the gate cannot enforce that scoping — uniqueness is not enforced at
// the gate at all, see register-collections.mjs); `key` is marked `required` and
// `unique` so the shape is documented and the future native enforcement is a flag
// flip, but POC uniqueness lives in the registrar/UI.

export const register = {
  kind: 'collection',
  name: 'register',
  schema: [
    { name: 'key', type: 'text', required: true },
    { name: 'name', type: 'text', required: true },
    // — protocol metadata (consumed by the poller) —
    { name: 'address', type: 'number', required: true },
    { name: 'fn_code', type: 'text' },
    { name: 'datatype', type: 'text' },
    { name: 'word_count', type: 'number' },
    { name: 'byte_order', type: 'text' },
    { name: 'scale', type: 'number' },
    { name: 'offset', type: 'number' },
    { name: 'signed', type: 'bool' },
    // — semantics & presentation (consumed by NHP/dashboards) —
    { name: 'unit', type: 'text' },
    { name: 'quantity', type: 'text' },
    { name: 'history', type: 'bool', required: true },
    { name: 'chart_type', type: 'text' },
    { name: 'chart_group', type: 'text' },
    { name: 'precision', type: 'number' },
    // `alarm` is a free-form JSON threshold object — see module doc.
    { name: 'meter', type: 'relation' },
  ],
};
