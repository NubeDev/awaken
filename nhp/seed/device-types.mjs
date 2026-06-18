// The "extra devices" meter-type templates — beyond the power meters in
// meter-types.mjs. Two families, both stamped through the SAME meter/register
// pipeline (a meter-type's `registers[]` of register-defs; DOMAIN-MODEL §meter-type):
//
//  1. LoRa sensors — battery-powered radios "matched to a gateway" by sitting on
//     that gateway's `lora` network (net_type/protocol 'lora', enums.mjs). Every
//     LoRa device carries a `battery` register with a LOW-battery alarm
//     (direction:'below' — fires as the value DROPS, field-config.ts). Sub-types:
//       - lora-pulse-water     : a pulse input metering water volume (litres)
//       - lora-pulse-electric  : a pulse input metering electrical energy (kWh)
//       - lora-temp            : a temperature sensor
//       - lora-co2             : a CO₂ sensor
//       - lora-co              : a CO sensor
//     LoRa registers are not Modbus-addressed; they are decoded from the uplink
//     payload, marked fn_code:'lora_uplink' with `address` a logical channel.
//
//  2. Modbus IO — generic Modbus I/O points on a 485/ethernet bus, same contract
//     as a power meter's registers (address + fn_code + datatype the poller reads):
//       - modbus-pulse   : an electrical pulse input (register + scale → energy),
//                          exactly like adding a power meter's energy register
//       - modbus-coil    : an on/off coil — read (read_coil) AND write (write_coil)
//       - modbus-holding : a holding register — read (read_holding) AND write
//                          (write_holding)
//
// Enum-valued fields use the allowed sets in nhp/collections/enums.mjs (enforced
// client-side at write). `alarm` is free-form JSON on the register-def (passes
// through the gate undeclared, like the power-meter alarms).

// --- LoRa register-def helpers ---

// The battery register every LoRa device carries. State-of-charge %, low-battery
// alarm: warn ≤30, critical ≤15 — a 'below' ramp (fires as charge DROPS). This is
// the low-battery requirement; a seeded device biased low trips it (history.mjs).
const battery = (channel = 0) => ({
  key: 'battery',
  name: 'Battery',
  address: channel,
  fn_code: 'lora_uplink',
  datatype: 'uint16',
  word_count: 1,
  byte_order: 'big',
  scale: 1,
  offset: 0,
  signed: false,
  unit: '%',
  quantity: 'battery',
  // a slow-moving state-of-charge reads best as a live tile, not a trend
  history: false,
  chart_type: 'stat',
  chart_group: 'battery',
  precision: 0,
  alarm: {
    direction: 'below',
    thresholds: [
      { value: null, severity: 'ok' },
      { value: 30, severity: 'warning' },
      { value: 15, severity: 'critical' },
    ],
    for: '15m',
  },
})

// A LoRa pulse input — a dry-contact pulse channel decoded into a cumulative
// total (water volume or electrical energy). `scale` converts pulses→unit.
const loraPulse = (key, name, unit, quantity, group, scale, channel = 1) => ({
  key,
  name,
  address: channel,
  fn_code: 'lora_uplink',
  datatype: 'uint32',
  word_count: 2,
  byte_order: 'big',
  scale,
  offset: 0,
  signed: false,
  unit,
  quantity,
  history: true,
  // a cumulative total reads best as a bar of interval deltas (like energy)
  chart_type: 'bar',
  chart_group: group,
  precision: 1,
})

// A LoRa scalar sensor (temp/CO₂/CO) with an optional HIGH alarm ramp (the
// default 'above' direction). `alarm` omitted ⇒ no ramp.
const loraSensor = (key, name, unit, quantity, group, precision, channel, alarm) => ({
  key,
  name,
  address: channel,
  fn_code: 'lora_uplink',
  datatype: 'float32',
  word_count: 2,
  byte_order: 'big',
  scale: 1,
  offset: 0,
  signed: true,
  unit,
  quantity,
  history: true,
  chart_type: 'line',
  chart_group: group,
  precision,
  ...(alarm ? { alarm } : {}),
})

// --- Modbus-IO register-def helpers ---

// A Modbus pulse-input register — read a holding register and scale it to energy,
// identical in shape to a power meter's energy register (the user's "same as
// adding a power meter").
const modbusPulse = (address) => ({
  key: 'pulse_energy',
  name: 'Pulse Energy',
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
  chart_type: 'bar',
  chart_group: 'energy',
  precision: 1,
})

// An on/off coil — read AND write. The poller reads its state with read_coil; an
// operator writes it with write_coil. Two register-defs share one coil address:
// `*_state` (readback, history) and `*_cmd` (the write point).
const coilState = (key, name, address) => ({
  key: `${key}_state`,
  name: `${name} (state)`,
  address,
  fn_code: 'read_coil',
  datatype: 'bool',
  word_count: 1,
  byte_order: 'big',
  scale: 1,
  offset: 0,
  signed: false,
  unit: '',
  quantity: 'state',
  history: true,
  chart_type: 'stat',
  chart_group: key,
  precision: 0,
})

const coilCmd = (key, name, address) => ({
  key: `${key}_cmd`,
  name: `${name} (command)`,
  address,
  fn_code: 'write_coil',
  datatype: 'bool',
  word_count: 1,
  byte_order: 'big',
  scale: 1,
  offset: 0,
  signed: false,
  unit: '',
  quantity: 'state',
  // a write point has no trend to keep
  history: false,
  chart_type: 'stat',
  chart_group: key,
  precision: 0,
})

// A read/write holding register — a generic analog setpoint/readback pair.
const holdingRead = (address) => ({
  key: 'setpoint_read',
  name: 'Setpoint (read)',
  address,
  fn_code: 'read_holding',
  datatype: 'float32',
  word_count: 2,
  byte_order: 'big',
  scale: 1,
  offset: 0,
  signed: true,
  unit: '',
  quantity: 'state',
  history: true,
  chart_type: 'line',
  chart_group: 'setpoint',
  precision: 1,
})

const holdingWrite = (address) => ({
  key: 'setpoint_write',
  name: 'Setpoint (write)',
  address,
  fn_code: 'write_holding',
  datatype: 'float32',
  word_count: 2,
  byte_order: 'big',
  scale: 1,
  offset: 0,
  signed: true,
  unit: '',
  quantity: 'state',
  history: false,
  chart_type: 'stat',
  chart_group: 'setpoint',
  precision: 1,
})

// High-alarm ramps for the environmental sensors (default 'above' direction).
// Temperature: a switch/comms-room over-temp alarm (warn ≥35 °C, critical ≥40).
const TEMP_ALARM = {
  thresholds: [
    { value: null, severity: 'ok' },
    { value: 35, severity: 'warning' },
    { value: 40, severity: 'critical' },
  ],
  for: '5m',
}
// CO₂: indoor-air-quality (warn ≥1000 ppm, critical ≥2000 — ASHRAE-ish).
const CO2_ALARM = {
  thresholds: [
    { value: null, severity: 'ok' },
    { value: 1000, severity: 'warning' },
    { value: 2000, severity: 'critical' },
  ],
  for: '5m',
}
// CO: life-safety (warn ≥35 ppm, critical ≥100 — carpark ventilation trigger).
const CO_ALARM = {
  thresholds: [
    { value: null, severity: 'ok' },
    { value: 35, severity: 'warning' },
    { value: 100, severity: 'critical' },
  ],
  for: '1m',
}

// --- the meter-type templates ---

const loraType = (key, name, sensorRegisters) => ({
  kind: 'meter-type',
  key,
  name,
  manufacturer: 'NHP',
  version: 1,
  registers: [...sensorRegisters, battery()],
});

export const DEVICE_TYPES = [
  // LoRa pulse inputs (water + electrical)
  loraType('nhp-lora-pulse-water', 'LoRa Pulse — Water', [
    loraPulse('water_volume', 'Water Volume', 'm³', 'volume', 'volume', 0.001),
  ]),
  loraType('nhp-lora-pulse-electric', 'LoRa Pulse — Electrical', [
    loraPulse('pulse_energy', 'Pulse Energy', 'kWh', 'energy', 'energy', 0.1),
  ]),
  // LoRa environmental sensors
  loraType('nhp-lora-temp', 'LoRa Temperature', [
    loraSensor('temperature', 'Temperature', '°C', 'temperature', 'temperature', 1, 1, TEMP_ALARM),
  ]),
  loraType('nhp-lora-co2', 'LoRa CO₂', [
    loraSensor('co2', 'CO₂', 'ppm', 'co2', 'co2', 0, 1, CO2_ALARM),
  ]),
  loraType('nhp-lora-co', 'LoRa CO', [
    loraSensor('co', 'CO', 'ppm', 'co', 'co', 1, 1, CO_ALARM),
  ]),

  // Modbus IO
  {
    kind: 'meter-type',
    key: 'nhp-modbus-pulse',
    name: 'Modbus Pulse Input',
    manufacturer: 'NHP',
    version: 1,
    registers: [modbusPulse(4000)],
  },
  {
    kind: 'meter-type',
    key: 'nhp-modbus-coil',
    name: 'Modbus On/Off Coil',
    manufacturer: 'NHP',
    version: 1,
    // one coil, exposed as a read state + a write command
    registers: [coilState('relay', 'Relay', 0), coilCmd('relay', 'Relay', 0)],
  },
  {
    kind: 'meter-type',
    key: 'nhp-modbus-holding',
    name: 'Modbus Holding Register',
    manufacturer: 'NHP',
    version: 1,
    registers: [holdingRead(4100), holdingWrite(4100)],
  },
];
