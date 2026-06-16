// The closed enum allowed-sets for NHP collection fields.
//
// rubix has no native Select/enum FieldType (see nhp/docs/OVERVIEW.md gap #1),
// and — verified against the unmodified backend — its gate validate step only
// enforces `required` + field TYPE, never a `writeRule`/allowed-set
// (rubix/crates/rubix-gate/src/command/validate.rs only calls
// CollectionDef::validate, and rubix/crates/rubix-core/src/collection/def.rs
// preserves writeRule as raw JSON it never evaluates). So every closed enum is
// modelled as a plain `text` field, and the allowed set lives HERE as data: the
// registrar enforces it on its check writes and the UI dropdown (WS-04/05) will
// constrain input. This file is the single source of truth for those sets so the
// definitions, the registrar checks, and the future UI all agree.
//
// See nhp/docs/DOMAIN-MODEL.md for where each enum is used.

export const NET_TYPE = ['485', 'ethernet'];
export const PROTOCOL = ['modbus'];

export const FN_CODE = [
  'read_holding',
  'read_input',
  'read_coil',
  'read_discrete',
  'write_holding',
  'write_coil',
];

export const DATATYPE = [
  'int16',
  'uint16',
  'int32',
  'uint32',
  'float32',
  'float64',
  'bool',
];

export const BYTE_ORDER = ['big', 'little', 'big_swap', 'little_swap'];

export const CHART_TYPE = ['line', 'bar', 'area', 'stat', 'gauge', 'table'];

// Poller-owned status (DOMAIN-MODEL "Status fields are poller-owned").
export const STATUS = ['online', 'offline', 'unknown'];

// The closed enums keyed by `collection.field`, so the registrar can check a
// candidate value against the right set without per-field code.
export const ENUMS = {
  'network.net_type': NET_TYPE,
  'network.protocol': PROTOCOL,
  'gateway.status': STATUS,
  'meter.status': STATUS,
  'register.fn_code': FN_CODE,
  'register.datatype': DATATYPE,
  'register.byte_order': BYTE_ORDER,
  'register.chart_type': CHART_TYPE,
};
