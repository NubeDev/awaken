// Synthetic trailing history for a register, the NHP stand-in for the (not-built)
// polling service. NHP never polls (OVERVIEW) — the seed plays the poller and
// writes time-series so dashboards (WS-07) have a trend to draw.
//
// One sample per hour over a trailing window. A sample is now a lean
// `{ at, value }` on the time-series DATA plane (rubix's `reading` table,
// READINGS-TIMESERIES.md): `at` IS the measurement instant the query layer buckets
// on, no longer stuffed in `content.ts` to dodge the gate's `created` stamp. The
// duplicated metadata (kind/meter/register/quantity/unit) is gone — the `series`
// (the register record id) carries it, named once per batch. Values are
// DETERMINISTIC (a fixed wave keyed off the register, not randomness) so a re-seed
// is reproducible; the deterministic (series, at) id then makes it idempotent.

// Hours of trailing history per register (one sample per hour). 48h gives the
// dashboard a couple of days of trend without bloating the seed.
const HISTORY_HOURS = 48;

// The wave a register oscillates around, derived from its quantity so each series
// looks physically plausible. `cumulative` series ramp (an energy total) instead
// of oscillating. base/swing are sensible defaults per quantity.
function profile(register) {
  switch (register.quantity) {
    case 'voltage':
      return { base: 230, swing: 6, cumulative: false };
    case 'current':
      return { base: 12, swing: 4, cumulative: false };
    case 'power':
      return { base: 8, swing: 3, cumulative: false };
    case 'energy':
      return { base: 1000, swing: 8, cumulative: true }; // +~8 kWh/hour
    case 'frequency':
      return { base: 50, swing: 0.05, cumulative: false };
    case 'power_factor':
      // A bounded 0..1 ratio — sit near unity (0.93..0.97) like a healthy load.
      return { base: 0.95, swing: 0.02, cumulative: false };
    // --- LoRa sensor / Modbus-IO quantities (device-types.mjs) ---
    case 'temperature':
      // A room/switch-room ambient — sit ~22 °C ±3 (well under the 35 °C warn).
      return { base: 22, swing: 3, cumulative: false };
    case 'co2':
      // Indoor air ~600 ppm ±150 (under the 1000 ppm warn).
      return { base: 600, swing: 150, cumulative: false };
    case 'co':
      // Clean air a few ppm (under the 35 ppm warn).
      return { base: 4, swing: 3, cumulative: false };
    case 'battery':
      // A healthy state-of-charge — sit high (~88% ±4) so it does NOT trip the
      // low-battery ramp unless a seeded device is biased down (spike < 0).
      return { base: 88, swing: 4, cumulative: false };
    case 'volume':
      // A water total — cumulative like energy, slower ramp (~0.3 m³/hour).
      return { base: 500, swing: 0.3, cumulative: true };
    case 'pulse':
      // A raw pulse count — cumulative.
      return { base: 0, swing: 5, cumulative: true };
    case 'state':
      // An on/off coil readback — flips around 0/1 (the wave crosses 0.5).
      return { base: 0.5, swing: 0.5, cumulative: false };
    default:
      return { base: 1, swing: 0.2, cumulative: false };
  }
}

// A small deterministic per-register phase so two registers of the same quantity
// don't draw identical lines. Hash the register key to a 0..1 fraction.
function phase(key) {
  let h = 0;
  for (let i = 0; i < key.length; i += 1) h = (h * 31 + key.charCodeAt(i)) % 997;
  return h / 997;
}

// The i-th sample value: a rising ramp for a cumulative total, else a deterministic
// oscillation within base ± swing (a coarse sine so the trend reads as wavy).
// `bias` shifts the whole wave up — used to drive a register over its alarm ramp
// for a handful of seeded meters (see historySamples' `spikeVolts`).
function sample(register, p, i, bias = 0) {
  const { base, swing, cumulative } = p;
  if (cumulative) return round2(base + swing * i);
  const ph = phase(register.key);
  const wave = Math.sin((i / 6 + ph) * Math.PI); // ~12h period
  return round2(base + bias + swing * wave);
}

const round2 = (n) => Math.round(n * 100) / 100;

// Build the trailing-window history records for one register as `content` objects,
// oldest first, ending at `now`. Each sample is a lean `{ at, value }`: the
// owning meter/register/quantity/unit are NOT repeated per row — they live on the
// series (the register record), reached once via the `series` the batch is
// appended under.
//
// A `history=false` register keeps no trend (DOMAIN-MODEL), but its gauge/stat
// tile still needs a CURRENT value — the live poller would PATCH one. We stand in
// for that with a SINGLE most-recent sample (the "last poll"), so e.g. Power
// Factor renders a number instead of an em-dash. So this returns the full window
// for history=true and exactly one point for history=false.
//
// `opts.spike` biases THIS register's wave by an additive amount so a chosen
// device crosses its alarm ramp — how the seed produces a few active alarms
// without a rule engine (the dashboards evaluate the latest value against the
// ramp). The amount is signed: positive lifts the wave over an 'above' ramp
// (voltage/CO/temperature), negative drops it under a 'below' ramp (low battery).
// The caller decides which device gets a spike and by how much (portfolio.mjs);
// this just applies it.
export function historySamples(register, now = new Date(), opts = {}) {
  const { spike = 0 } = opts;
  const bias = spike;
  const p = profile(register);
  if (!register.history) {
    // One latest reading so the live tile has a value; no trend persisted.
    return [{ at: now.toISOString(), value: sample(register, p, HISTORY_HOURS - 1, bias) }];
  }
  const samples = [];
  for (let i = 0; i < HISTORY_HOURS; i += 1) {
    const at = new Date(now.getTime() - (HISTORY_HOURS - 1 - i) * 3600_000);
    samples.push({ at: at.toISOString(), value: sample(register, p, i, bias) });
  }
  return samples;
}

export { HISTORY_HOURS };
