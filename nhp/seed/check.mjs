// Verify the seeded portfolio meets the WS-03 done-gate, by querying the running
// backend over the same HTTP records API the seed wrote through. Asserts the
// expected counts and the qualitative checks; exits non-zero on any miss so
// `make seed-check` gates the build.
//
// Done-gate (WS-03): 2 tenants, 4 sites, ≥1 meter-type with a full register set,
// meters tagged, history present for history=true registers.
//
//   node nhp/seed/check.mjs        (run after node nhp/seed/seed.mjs)

import { listRecords, getReadings } from '../collections/client.mjs';
import { METER_TYPES } from './meter-types.mjs';

let failures = 0;
const check = (ok, msg) => {
  console.log(`  ${ok ? 'PASS' : 'FAIL'}  ${msg}`);
  if (!ok) failures += 1;
};

async function run() {
  const [tenants, sites, gateways, networks, meters, registers, meterTypes] =
    await Promise.all([
      listRecords('tenant'),
      listRecords('site'),
      listRecords('gateway'),
      listRecords('network'),
      listRecords('meter'),
      listRecords('register'),
      listRecords('meter-type'),
    ]);

  // Counts the seed is expected to have created (the portfolio plan is fixed).
  check(tenants.length >= 2, `tenants ≥ 2 (got ${tenants.length})`);
  check(sites.length >= 4, `sites ≥ 4 (got ${sites.length})`);
  check(gateways.length >= 4, `gateways ≥ 4 (got ${gateways.length})`);
  check(networks.length >= 5, `networks ≥ 5 (got ${networks.length})`);
  check(meters.length >= 12, `meters ≥ 12 (got ${meters.length})`);

  // ≥1 meter-type with a full register set (the PM5560 carries the full 3-phase
  // set: V L1/L2/L3, A L1/L2/L3, kW, kWh, Hz, PF).
  const full = METER_TYPES.find((t) => t.key === 'acme-pm5560');
  const pm = meterTypes.find((r) => r.content?.key === 'acme-pm5560');
  check(Boolean(pm), 'meter-type acme-pm5560 present');
  const regCount = pm?.content?.registers?.length ?? 0;
  check(regCount === full.registers.length, `acme-pm5560 has full register set (${regCount})`);

  // Networks mix 485 + ethernet (DOMAIN-MODEL: a gateway carries both link types).
  const netTypes = new Set(networks.map((n) => n.content?.net_type));
  check(netTypes.has('485') && netTypes.has('ethernet'), `networks mix 485 + ethernet (${[...netTypes].join(', ')})`);

  // Meters are tagged with the standard hierarchy tags (content.tags — tags.mjs).
  const tagged = meters.filter((m) => {
    const tags = m.content?.tags ?? [];
    return tags.some((t) => t.startsWith('site:')) && tags.some((t) => t.startsWith('meter-type:'));
  });
  check(tagged.length === meters.length, `every meter tagged site:/meter-type: (${tagged.length}/${meters.length})`);

  // Meters carry the stamped version (DOMAIN-MODEL §versioning).
  const stamped = meters.filter((m) => typeof m.content?.meter_type_version === 'number');
  check(stamped.length === meters.length, `every meter stamped with meter_type_version (${stamped.length}/${meters.length})`);

  // Poller faked status/last_seen on every gateway + meter.
  const withStatus = [...gateways, ...meters].filter((r) => r.content?.status && r.content?.last_seen);
  check(
    withStatus.length === gateways.length + meters.length,
    `poller status+last_seen on every gateway/meter (${withStatus.length}/${gateways.length + meters.length})`,
  );
  check(
    gateways.some((g) => g.content?.status === 'offline'),
    'at least one gateway is offline (rollup has something to show)',
  );

  // History now lives in the `reading` DATA plane, not the `record` table. Read it
  // back through the windowed historian (`GET /readings`) per series — the series
  // IS the register record id — over a wide window covering the trailing 48h.
  const from = new Date(Date.now() - 60 * 86400_000).toISOString();
  const to = new Date(Date.now() + 86400_000).toISOString();
  let historyCount = 0;
  let firstSample = null;
  let noHistoryRegs = 0; // history=false registers (each should carry ONE point)
  let noHistoryWithValue = 0; // …and how many actually have their latest value
  let alarmingVoltages = 0; // voltage series whose LATEST reading crosses the ramp
  for (const reg of registers) {
    const rows = await getReadings(reg.id, from, to);
    if (reg.content?.history) {
      historyCount += rows.length;
      if (!firstSample && rows.length) firstSample = rows[0];
    } else {
      noHistoryRegs += 1;
      if (rows.length >= 1) noHistoryWithValue += 1;
    }
    // Alarm check: the dashboards evaluate a register's LATEST value against its
    // ramp; mirror that to confirm the seed produces active alarms (warn ≥250 V).
    if (reg.content?.quantity === 'voltage' && rows.length) {
      const latest = rows.reduce((a, b) => (Date.parse(b.at) > Date.parse(a.at) ? b : a));
      if (latest.value >= 250) alarmingVoltages += 1;
    }
  }
  check(historyCount > 0, `readings present for history=true registers (got ${historyCount})`);
  // A reading is lean: `at` (the measurement instant), `series`, `value`.
  check(
    Boolean(firstSample?.at && firstSample?.series && firstSample?.value !== undefined),
    'a reading carries at + series + value',
  );
  // `series` is the register RECORD id — a direct `series === register.id` join, no
  // string splitting (the UI relies on this).
  check(
    Boolean(firstSample) && registers.some((r) => r.id === firstSample.series),
    'a reading.series matches a register record id (direct join)',
  );
  // A history=false register keeps no trend, but the seed stands in for the live
  // poller with exactly ONE latest reading so its gauge/stat tile has a value
  // (e.g. Power Factor renders a number, not an em-dash).
  check(
    noHistoryRegs > 0 && noHistoryWithValue === noHistoryRegs,
    `every history=false register has a latest value (${noHistoryWithValue}/${noHistoryRegs})`,
  );
  // The seed spikes a few scattered meters' voltage over the alarm ramp so the
  // dashboards have active alarms to roll up (warn ≥250, critical ≥253).
  check(
    alarmingVoltages > 0,
    `at least one voltage series is in alarm (got ${alarmingVoltages})`,
  );

  console.log(failures === 0 ? 'seed check: all passed' : `seed check: ${failures} failed`);
  process.exit(failures === 0 ? 0 : 1);
}

run().catch((err) => {
  console.error(err.message);
  process.exit(1);
});
