// Register the 7 NHP collection definitions as rubix `kind:"collection"` records.
//
// Idempotent: a collection record already present (matched by its `name`) is left
// as-is, so a re-run is a no-op. rubix has no "update collection" semantics we
// need here for the POC — defining a collection that already exists would create
// a second record, so we skip instead. (A real edit path is WS-04 admin work.)
//
// Run: node nhp/collections/register-collections.mjs   (env in client.mjs)

import { DEFINITIONS } from './definitions.mjs';
import { createRecord, listRecords } from './client.mjs';

export async function registerCollections() {
  const existing = await listRecords('collection');
  const present = new Set(
    existing.map((r) => r.content?.name).filter(Boolean),
  );

  const results = [];
  for (const def of DEFINITIONS) {
    if (present.has(def.name)) {
      results.push({ name: def.name, status: 'exists' });
      continue;
    }
    const res = await createRecord(def);
    if (!res.ok) {
      throw new Error(
        `registering collection \`${def.name}\` failed: ${res.status} ${JSON.stringify(res.body)}`,
      );
    }
    results.push({ name: def.name, status: 'created' });
  }
  return results;
}

// Allow `node register-collections.mjs` to run the registrar directly.
if (import.meta.url === `file://${process.argv[1]}`) {
  registerCollections()
    .then((rows) => {
      for (const r of rows) console.log(`  ${r.status.padEnd(8)} ${r.name}`);
      console.log(`registered ${rows.length} NHP collections`);
    })
    .catch((err) => {
      console.error(err.message);
      process.exit(1);
    });
}
