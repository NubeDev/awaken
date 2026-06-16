// Assert the DASHBOARD DATA PIPELINE returns real rows over the same `/records`
// API the UI's dashboards read through (nhp/ui/src/features/dashboards/query/
// batch.ts fetches each kind whole and filters by `content.tags` client-side —
// rubix has no /query/batch, see WS-07/TODOs). This is the headless equivalent of
// "open a dashboard and see history + a status rollup": it proves the pipeline the
// UI depends on returns rows, without a browser.
//
// Exits non-zero on any empty result so `make smoke` gates on it.
//
//   node nhp/seed/records-check.mjs        (run after node nhp/seed/seed.mjs)

import { listRecords, getReadings } from '../collections/client.mjs';

let failures = 0;
const check = (ok, msg) => {
  console.log(`  ${ok ? 'PASS' : 'FAIL'}  ${msg}`);
  if (!ok) failures += 1;
};

async function run() {
  // The dashboard auto-builder walks these record kinds; history now comes from
  // the `reading` DATA plane (via the windowed historian), not a `history` kind.
  // If any comes back empty the boards render nothing, so each must have rows.
  const [tenants, sites, gateways, meters, registers] = await Promise.all([
    listRecords('tenant'),
    listRecords('site'),
    listRecords('gateway'),
    listRecords('meter'),
    listRecords('register'),
  ]);

  check(tenants.length > 0, `dashboard tenant list returns rows (${tenants.length})`);
  check(sites.length > 0, `dashboard site list returns rows (${sites.length})`);
  check(gateways.length > 0, `dashboard gateway list returns rows (${gateways.length})`);
  check(meters.length > 0, `dashboard meter list returns rows (${meters.length})`);
  check(registers.length > 0, `dashboard register list returns rows (${registers.length})`);

  // History series now read through `GET /readings` per series (the register
  // record id). Collect a sample across the history=true registers — enough to
  // prove the windowed historian returns a trend.
  const from = new Date(Date.now() - 60 * 86400_000).toISOString();
  const to = new Date(Date.now() + 86400_000).toISOString();
  const historyRegs = registers.filter((r) => r.content?.history);
  let readings = [];
  for (const reg of historyRegs) {
    readings = readings.concat(await getReadings(reg.id, from, to));
    if (readings.length > 200) break; // enough to prove the pipeline returns a trend
  }
  check(readings.length > 0, `dashboard reading series returns rows (${readings.length})`);

  // The status rollup needs `content.status` on gateways (site cards roll up
  // gateway status; degraded if any offline). Prove the field the rollup reads is
  // present and at least one gateway is offline (so the rollup shows degraded).
  const statused = gateways.filter((g) => g.content?.status);
  check(statused.length === gateways.length, `every gateway carries status for the rollup (${statused.length}/${gateways.length})`);
  check(gateways.some((g) => g.content?.status === 'offline'), 'a gateway is offline so the rollup shows degraded');

  // A meter trend chart joins readings by `series === register.id`, then the
  // register to its meter via `content.meter`. Prove that join resolves to a meter.
  const meterIds = new Set(meters.map((m) => m.id));
  const regById = new Map(registers.map((r) => [r.id, r.content]));
  const joined = readings.filter((rd) => meterIds.has(regById.get(rd.series)?.meter));
  check(joined.length > 0, `readings join series→register→meter for a trend chart (${joined.length} samples)`);

  console.log(failures === 0 ? 'records check: all passed' : `records check: ${failures} failed`);
  process.exit(failures === 0 ? 0 : 1);
}

run().catch((err) => {
  console.error(err.message);
  process.exit(1);
});
