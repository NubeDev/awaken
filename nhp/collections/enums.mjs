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

// `485`/`ethernet` are Modbus link types; `lora` is the LoRaWAN radio link a
// gateway hosts for battery-powered sensors (a LoRa device is "matched to a
// gateway" by sitting on that gateway's `lora` network). DOMAIN-MODEL §network.
export const NET_TYPE = ['485', 'ethernet', 'lora'];
// `modbus` is the field-bus protocol; `lora` (LoRaWAN) carries the sensor uplinks
// the gateway forwards. A `lora` network always pairs with `protocol: 'lora'`.
export const PROTOCOL = ['modbus', 'lora'];

// Register access primitives. The `read_*`/`write_*` codes are Modbus function
// codes (consumed by the Modbus poller). `lora_uplink` is the LoRa stand-in: a
// LoRa register is not polled by address but decoded from the device's uplink
// payload, so it carries a logical channel in `address` and this fn_code marks it
// as payload-sourced rather than a Modbus read.
export const FN_CODE = [
  'read_holding',
  'read_input',
  'read_coil',
  'read_discrete',
  'write_holding',
  'write_coil',
  'lora_uplink',
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

// The closed set of register `quantity` values NHP renders (icons/colours in the
// dashboard `metric-style`, grouping, and the cross-meter rollups). Power-meter
// electrical quantities plus the new sensor/IO quantities the LoRa + Modbus-IO
// devices introduce. Open-ended at the gate (it's a plain text field); this set
// is what the UI styles — an unknown quantity still works, it just falls back.
export const QUANTITY = [
  // electrical (power meters)
  'voltage',
  'current',
  'power',
  'energy',
  'frequency',
  'power_factor',
  // environmental sensors (LoRa)
  'temperature',
  'co2',
  'co',
  'battery',
  // pulse / IO
  'volume', // water pulse → litres/m³
  'pulse', // raw pulse count (electrical/util pulse input)
  'state', // on/off coil readback
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
  'register.quantity': QUANTITY,
};
