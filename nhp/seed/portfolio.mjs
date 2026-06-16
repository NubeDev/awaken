// Build the mock NHP portfolio through the gate (SEED.md): write the meter-types,
// then walk PORTFOLIO writing tenant→site→gateway→network→meter→register records,
// stamping each meter from its type, faking poller status/last_seen, and
// back-filling history for history=true registers.
//
// Everything is written over the SAME rubix HTTP records API the collections use
// (reuses nhp/collections/client.mjs) and the SAME standard tags WS-06/07 import
// (reuses nhp/seed/tags.mjs). Records carry their parent's record id in the
// relation field (DOMAIN-MODEL: child stores parent id) and their standard tags in
// `content.tags` (tags.mjs explains why content, not graph edges).
//
// Idempotent-ish (SEED.md): every record carries a stable `key`; before writing we
// list the kind and skip a key already present, so a re-seed against a populated
// store is a no-op rather than a duplicate. Matches the collections registrar.

import { createRecord, listRecords, appendReadings } from '../collections/client.mjs';
import { METER_TYPES } from './meter-types.mjs';
import { PORTFOLIO } from './portfolio-plan.mjs';
import { historySamples } from './history.mjs';
import { pollerFields } from './poller-status.mjs';
import {
  siteTags,
  gatewayTags,
  networkTags,
  meterTags,
  registerTags,
} from './tags.mjs';

// Create a record unless one of its kind already carries the same `key`. Returns
// `{ id, created }` — the record id (existing or new) so children can link to it,
// and whether THIS run created it (the caller back-fills history only for a
// freshly-created register, so a re-seed doesn't duplicate the time-series). Throws
// on a real write failure so the seed fails loud.
async function upsert(kind, content, existingByKey) {
  const found = existingByKey.get(content.key);
  if (found) return { id: found, created: false };
  const res = await createRecord({ kind, ...content });
  if (!res.ok) {
    throw new Error(
      `seed ${kind} \`${content.key}\` failed: ${res.status} ${JSON.stringify(res.body)}`,
    );
  }
  existingByKey.set(content.key, res.body.id);
  return { id: res.body.id, created: true };
}

// Index existing records of a kind by their content `key` → id, so upsert can skip
// what's already there (idempotent re-seed).
async function indexByKey(kind) {
  const records = await listRecords(kind);
  const map = new Map();
  for (const r of records) {
    const key = r.content?.key;
    if (key) map.set(key, r.id);
  }
  return map;
}

export async function seedPortfolio({ log = () => {} } = {}) {
  const now = new Date();
  const tally = { tenants: 0, sites: 0, gateways: 0, networks: 0, meters: 0, registers: 0, history: 0 };

  // --- meter-types first (meters stamp from them) ---
  const mtIndex = await indexByKey('meter-type');
  const typeById = new Map(); // key → { id, version, registers }
  for (const mt of METER_TYPES) {
    const { registers, kind, ...fields } = mt;
    const { id } = await upsert('meter-type', { ...fields, registers }, mtIndex);
    typeById.set(mt.key, { id, version: mt.version, registers });
  }
  log(`  meter-types: ${METER_TYPES.length}`);

  // Pre-load the indexes for every kind once (re-seed idempotency).
  const idx = {
    tenant: await indexByKey('tenant'),
    site: await indexByKey('site'),
    gateway: await indexByKey('gateway'),
    network: await indexByKey('network'),
    meter: await indexByKey('meter'),
    register: await indexByKey('register'),
  };

  // A monotonically rising ordinal across all devices, so the faked poller marks a
  // stable, spread-out subset offline (poller-status.mjs).
  let deviceOrdinal = 0;

  for (const tenant of PORTFOLIO) {
    const { id: tenantId } = await upsert(
      'tenant',
      { key: tenant.key, name: tenant.name, namespace: tenant.namespace, tags: [] },
      idx.tenant,
    );
    tally.tenants += 1;

    for (const site of tenant.sites) {
      const ctx = { tenant: tenant.key };
      const { id: siteId } = await upsert(
        'site',
        {
          key: site.key,
          name: site.name,
          tenant: tenantId,
          address: site.address,
          timezone: site.timezone,
          geo: site.geo,
          tags: siteTags(ctx),
        },
        idx.site,
      );
      tally.sites += 1;

      for (const gw of site.gateways) {
        const gwCtx = { tenant: tenant.key, site: site.key };
        const { id: gwId } = await upsert(
          'gateway',
          {
            key: gw.key,
            name: gw.name,
            site: siteId,
            model: gw.model,
            host: gw.host,
            ...pollerFields(deviceOrdinal++, now),
            tags: gatewayTags(gwCtx),
          },
          idx.gateway,
        );
        tally.gateways += 1;

        for (const net of gw.networks) {
          const netCtx = { tenant: tenant.key, site: site.key, gateway: gw.key };
          const { id: netId } = await upsert(
            'network',
            {
              key: net.key,
              name: net.name,
              gateway: gwId,
              net_type: net.net_type,
              protocol: net.protocol,
              max_devices: net.max_devices,
              params: net.params,
              tags: networkTags(netCtx),
            },
            idx.network,
          );
          tally.networks += 1;

          for (const m of net.meters) {
            const type = typeById.get(m.mt);
            if (!type) throw new Error(`meter ${m.key} references unknown type ${m.mt}`);
            const { id: meterId } = await upsert(
              'meter',
              {
                key: m.key,
                name: m.name,
                network: netId,
                meter_type: type.id,
                meter_type_version: type.version, // stamp-on-create (DOMAIN-MODEL §versioning)
                address: m.addr,
                ...pollerFields(deviceOrdinal++, now),
                tags: meterTags({
                  tenant: tenant.key,
                  site: site.key,
                  gateway: gw.key,
                  network: net.key,
                  meterType: m.mt,
                }),
              },
              idx.meter,
            );
            tally.meters += 1;

            // Stamp the meter's registers from the type's register-defs, then
            // back-fill history for the history=true ones.
            const regCtx = {
              tenant: tenant.key,
              site: site.key,
              gateway: gw.key,
              network: net.key,
              meter: m.key,
            };
            for (const def of type.registers) {
              const regKey = `${m.key}--${def.key}`; // unique per meter
              const { id: registerId } = await upsert(
                'register',
                {
                  ...def,
                  key: regKey,
                  meter: meterId,
                  tags: registerTags(regCtx, def),
                },
                idx.register,
              );
              tally.registers += 1;

              // Back-fill history for every history=true register, every run: the
              // deterministic (series, at) reading id makes a re-append an
              // idempotent no-op, so — unlike the old keyless record path — there
              // is no need to gate on "freshly created". `series` is the register
              // RECORD id; the samples are lean `{ at, value }`.
              const samples = historySamples(def, now);
              if (samples.length === 0) continue;
              const res = await appendReadings(registerId, samples);
              if (!res.ok) {
                throw new Error(
                  `seed readings ${regKey} failed: ${res.status} ${JSON.stringify(res.body)}`,
                );
              }
              tally.history += res.body?.appended ?? samples.length;
            }
          }
        }
      }
    }
  }

  return tally;
}
