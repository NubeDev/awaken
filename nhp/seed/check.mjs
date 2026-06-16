// Verify the seeded portfolio meets the WS-03 done-gate, by querying the running
// backend over the same HTTP records API the seed wrote through. Asserts the
// expected counts and the qualitative checks; exits non-zero on any miss so
// `make seed-check` gates the build.
//
// Done-gate (WS-03): 2 tenants, 4 sites, ≥1 meter-type with a full register set,
// meters tagged, history present for history=true registers.
//
//   node nhp/seed/check.mjs        (run after node nhp/seed/seed.mjs)

import { listRecords } from '../collections/client.mjs';
import { METER_TYPES } from './meter-types.mjs';

let failures = 0;
const check = (ok, msg) => {
  console.log(`  ${ok ? 'PASS' : 'FAIL'}  ${msg}`);
  if (!ok) failures += 1;
};

async function run() {
  const [tenants, sites, gateways, networks, meters, registers, meterTypes, history] =
    await Promise.all([
      listRecords('tenant'),
      listRecords('site'),
      listRecords('gateway'),
      listRecords('network'),
      listRecords('meter'),
      listRecords('register'),
      listRecords('meter-type'),
      listRecords('history'),
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

  // History present for history=true registers. The PM5560 has 9 history=true
  // registers (all but power_factor) × 12 PM5560 meters worth of series… simplest
  // assertion: there is history at all, and a sample carries ts + value + register.
  check(history.length > 0, `history rows present (got ${history.length})`);
  const sample = history[0]?.content;
  check(
    Boolean(sample?.ts && sample?.register && sample?.value !== undefined),
    'a history sample carries ts + register + value',
  );
  // Every history row's register is a history=true register (poller never persists
  // a no-history register).
  const historyRegisters = new Set(history.map((h) => h.content?.register?.split('--').pop()));
  const noHistoryKeys = new Set(
    METER_TYPES.flatMap((t) => t.registers).filter((r) => !r.history).map((r) => r.key),
  );
  const leaked = [...historyRegisters].filter((k) => noHistoryKeys.has(k));
  check(leaked.length === 0, `no history written for history=false registers (leaked: ${leaked.join(', ') || 'none'})`);

  console.log(failures === 0 ? 'seed check: all passed' : `seed check: ${failures} failed`);
  process.exit(failures === 0 ? 0 : 1);
}

run().catch((err) => {
  console.error(err.message);
  process.exit(1);
});
