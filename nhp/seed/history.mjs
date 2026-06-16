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
function sample(register, p, i) {
  const { base, swing, cumulative } = p;
  if (cumulative) return round2(base + swing * i);
  const ph = phase(register.key);
  const wave = Math.sin((i / 6 + ph) * Math.PI); // ~12h period
  return round2(base + swing * wave);
}

const round2 = (n) => Math.round(n * 100) / 100;

// Build the trailing-window history records for one register as `content` objects,
// oldest first, ending at `now`. Each sample is a lean `{ at, value }`: the
// owning meter/register/quantity/unit are NOT repeated per row — they live on the
// series (the register record), reached once via the `series` the batch is
// appended under. Returns [] when the register keeps no history (DOMAIN-MODEL:
// only `history=true` registers persist).
export function historySamples(register, now = new Date()) {
  if (!register.history) return [];
  const p = profile(register);
  const samples = [];
  for (let i = 0; i < HISTORY_HOURS; i += 1) {
    const at = new Date(now.getTime() - (HISTORY_HOURS - 1 - i) * 3600_000);
    samples.push({ at: at.toISOString(), value: sample(register, p, i) });
  }
  return samples;
}

export { HISTORY_HOURS };
