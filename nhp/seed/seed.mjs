// The one-command NHP seed entrypoint (WS-03, SEED.md): against a RUNNING rubix
// backend, register the NHP collections then write the mock portfolio + faked
// poller data. Reuses the collections registrar and the portfolio builder; this
// file only orders them and prints a tally.
//
// Dependency order (CRITICAL, see WS-03 + Makefile): rubix must be booted with
// --seed-dev first — that creates the `acme` namespace and the operator principal
// this seed authenticates as (nhp/collections/client.mjs). Then collections must
// exist before the portfolio validates against them. So: collections → portfolio.
//
//   node nhp/seed/seed.mjs        (env: RUBIX_BASE / RUBIX_SUBJECT / RUBIX_SECRET)
//
// Off by default at the Makefile level (SEED=1); this script always seeds when run.

import { registerCollections } from '../collections/register-collections.mjs';
import { seedPortfolio } from './portfolio.mjs';

async function main() {
  console.log('NHP seed — registering collections…');
  const collections = await registerCollections();
  const created = collections.filter((c) => c.status === 'created').length;
  console.log(`  collections: ${collections.length} (${created} created, ${collections.length - created} existing)`);

  console.log('NHP seed — writing portfolio + faked poller data…');
  const t = await seedPortfolio({ log: (m) => console.log(m) });

  console.log('NHP seed — done:');
  console.log(
    `  ${t.tenants} tenants, ${t.sites} sites, ${t.gateways} gateways, ` +
      `${t.networks} networks, ${t.meters} meters, ${t.registers} registers, ` +
      `${t.history} history rows`,
  );
}

main().catch((err) => {
  console.error(`seed failed: ${err.message}`);
  process.exit(1);
});
