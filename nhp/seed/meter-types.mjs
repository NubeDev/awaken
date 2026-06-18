// The seed's meter-type templates — the admin-defined register maps a meter is
// stamped from (DOMAIN-MODEL.md §meter-type, §register). Each register-def carries
// the full Modbus contract the (external) poller consumes plus the
// semantics/presentation NHP renders: address/fn_code/datatype/word_count/
// byte_order/scale/offset/unit/quantity/history/chart_type/chart_group/precision
// and an optional `alarm` threshold ramp (DOMAIN-MODEL §alarms).
//
// Two types so the portfolio exercises stamping from more than one template:
//  - acme-pm5560  : a full 3-phase power meter (V L1/L2/L3, A L1/L2/L3, kW, kWh,
//                   Hz, PF) — the "full register set" the done-gate checks for.
//  - acme-em24    : a compact single-line energy meter (kW, kWh, PF).
//
// `registers` is free-form JSON on the meter-type record (WS-02: rubix has no json
// FieldType; undeclared content passes through). The enum-valued fields use the
// allowed sets from nhp/collections/enums.mjs (enforced client-side at write).

// A 3-phase voltage register-def (L1/L2/L3 share shape, differ by line/address).
const voltage = (line, address) => ({
  key: `voltage_l${line}`,
  name: `Voltage L${line}`,
  address,
  fn_code: 'read_holding',
  datatype: 'float32',
  word_count: 2,
  byte_order: 'big',
  scale: 1,
  offset: 0,
  signed: false,
  unit: 'V',
  quantity: 'voltage',
  history: true,
  chart_type: 'line',
  chart_group: 'voltage',
  precision: 1,
  // Nominal 230 V line; warn over 250, critical over 253 (EN 50160-ish).
  alarm: {
    thresholds: [
      { value: null, severity: 'ok' },
      { value: 250, severity: 'warning' },
      { value: 253, severity: 'critical' },
    ],
    for: '5m',
  },
});

const current = (line, address) => ({
  key: `current_l${line}`,
  name: `Current L${line}`,
  address,
  fn_code: 'read_holding',
  datatype: 'float32',
  word_count: 2,
  byte_order: 'big',
  scale: 1,
  offset: 0,
  signed: false,
  unit: 'A',
  quantity: 'current',
  history: true,
  chart_type: 'line',
  chart_group: 'current',
  precision: 2,
});

const activePower = (address) => ({
  key: 'active_power',
  name: 'Active Power',
  address,
  fn_code: 'read_holding',
  datatype: 'float32',
  word_count: 2,
  byte_order: 'big',
  scale: 1,
  offset: 0,
  signed: true,
  unit: 'kW',
  quantity: 'power',
  history: true,
  chart_type: 'line',
  chart_group: 'power',
  precision: 2,
});

const energy = (address) => ({
  key: 'active_energy',
  name: 'Active Energy',
  address,
  fn_code: 'read_holding',
  datatype: 'uint32',
  word_count: 2,
  byte_order: 'big',
  scale: 0.1,
  offset: 0,
  signed: false,
  unit: 'kWh',
  quantity: 'energy',
  history: true,
  // a cumulative total reads best as a bar of interval deltas
  chart_type: 'bar',
  chart_group: 'energy',
  precision: 1,
});

const frequency = (address) => ({
  key: 'frequency',
  name: 'Frequency',
  address,
  fn_code: 'read_holding',
  datatype: 'float32',
  word_count: 2,
  byte_order: 'big',
  scale: 1,
  offset: 0,
  signed: false,
  unit: 'Hz',
  quantity: 'frequency',
  history: true,
  chart_type: 'line',
  chart_group: 'frequency',
  precision: 2,
});

const powerFactor = (address) => ({
  key: 'power_factor',
  name: 'Power Factor',
  address,
  fn_code: 'read_holding',
  datatype: 'float32',
  word_count: 2,
  byte_order: 'big',
  scale: 1,
  offset: 0,
  signed: true,
  unit: 'PF',
  quantity: 'power_factor',
  // a bounded 0..1 ratio is a live tile, not a trend — history off
  history: false,
  chart_type: 'gauge',
  chart_group: 'power_factor',
  precision: 3,
});

// Full 3-phase power meter — the "full register set" the done-gate verifies.
const pm5560 = {
  kind: 'meter-type',
  key: 'acme-pm5560',
  name: 'Acme PM5560',
  manufacturer: 'Acme',
  version: 1,
  registers: [
    voltage(1, 3027),
    voltage(2, 3029),
    voltage(3, 3031),
    current(1, 3000),
    current(2, 3002),
    current(3, 3004),
    activePower(3060),
    energy(2700),
    frequency(3110),
    powerFactor(3084),
  ],
};

// Compact single-line energy meter — fewer registers, still a valid stamp source.
const em24 = {
  kind: 'meter-type',
  key: 'acme-em24',
  name: 'Acme EM24',
  manufacturer: 'Acme',
  version: 1,
  registers: [activePower(40), energy(52), powerFactor(46)],
};

export const METER_TYPES = [pm5560, em24];

// The "extra devices" — LoRa sensors + Modbus IO — are meter-type templates too
// (device-types.mjs), stamped through the same meter/register pipeline. The seed
// registers ALL_METER_TYPES so the portfolio can stamp meters from any of them.
import { DEVICE_TYPES } from './device-types.mjs';

export const ALL_METER_TYPES = [...METER_TYPES, ...DEVICE_TYPES];
