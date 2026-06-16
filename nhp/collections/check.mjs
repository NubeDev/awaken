// Verify the 7 NHP collections end-to-end against the unmodified rubix backend:
// for every kind, one VALID record writes, and one INVALID record is rejected —
// either by the gate (missing required / wrong type) or by the client-side
// enforcement layer (bad enum / dup unique / device limit) that covers the rules
// rubix's gate does not (see enforce.mjs). Exits non-zero if any case misbehaves.
//
// Run after register-collections.mjs against the same server.
//   node nhp/collections/check.mjs

import { DEFINITIONS } from './definitions.mjs';
import { createRecord, listRecords } from './client.mjs';
import { clientViolations, deviceLimitViolation } from './enforce.mjs';
import { tenant } from './tenant.mjs';
import { network } from './network.mjs';
import { meter } from './meter.mjs';

const stamp = Date.now();
const k = (kind) => `chk-${kind}-${stamp}`;

// A well-formed record body per kind. Relations carry a placeholder id string
// (the gate does not verify the target exists — relation = id string). Enum
// fields carry an allowed value. `stamp` keeps keys unique across re-runs.
function validBodies() {
  return {
    tenant: { kind: 'tenant', key: k('tenant'), name: 'Check Tenant', namespace: 'chk' },
    site: { kind: 'site', key: k('site'), name: 'Check Site', tenant: 'rel-tenant' },
    gateway: { kind: 'gateway', key: k('gateway'), name: 'GW', site: 'rel-site', status: 'online' },
    network: {
      kind: 'network', key: k('network'), name: 'Net', gateway: 'rel-gw',
      net_type: '485', protocol: 'modbus', max_devices: 32,
    },
    'meter-type': { kind: 'meter-type', key: k('metertype'), name: 'PM5560', version: 1 },
    meter: {
      kind: 'meter', key: k('meter'), name: 'M1', network: 'rel-net',
      meter_type: 'rel-mt', address: 5, status: 'online',
    },
    register: {
      kind: 'register', key: k('register'), name: 'Voltage L1', address: 3027,
      fn_code: 'read_holding', datatype: 'float32', byte_order: 'big', history: true,
      chart_type: 'line', unit: 'V',
    },
  };
}

// An invalid body per kind + how it should be caught. `gate` cases omit a
// required field (rubix rejects on write); `client` cases carry a bad enum
// (rubix admits it — our enforce.mjs must catch it).
function invalidCases(valid) {
  return {
    // missing required `key` → gate 422
    tenant: { by: 'gate', body: { kind: 'tenant', name: 'No Key' } },
    // missing required `name` → gate 422
    site: { by: 'gate', body: { kind: 'site', key: k('site-bad'), tenant: 'rel-t' } },
    // bad enum `status` → client
    gateway: { by: 'client', body: { ...valid.gateway, key: k('gw-bad'), status: 'flapping' } },
    // bad enum `net_type` → client
    network: { by: 'client', body: { ...valid.network, key: k('net-bad'), net_type: 'wifi' } },
    // missing required `version` → gate 422
    'meter-type': { by: 'gate', body: { kind: 'meter-type', key: k('mt-bad'), name: 'X' } },
    // bad enum `status` → client (also exercises a meter)
    meter: { by: 'client', body: { ...valid.meter, key: k('meter-bad'), status: 'dead' } },
    // missing required `history` → gate 422
    register: {
      by: 'gate',
      body: { kind: 'register', key: k('reg-bad'), name: 'R', address: 1 },
    },
  };
}

let failures = 0;
const log = (ok, msg) => {
  console.log(`  ${ok ? 'PASS' : 'FAIL'}  ${msg}`);
  if (!ok) failures += 1;
};

async function run() {
  const valid = validBodies();
  const invalid = invalidCases(valid);

  console.log('valid writes (gate accepts a well-formed record per kind):');
  for (const def of DEFINITIONS) {
    const body = valid[def.name];
    const existing = await listRecords(def.name);
    const cv = clientViolations(def, body, existing);
    if (cv.length > 0) {
      log(false, `${def.name}: valid body hit client checks: ${cv.join('; ')}`);
      continue;
    }
    const res = await createRecord(body);
    log(res.ok, `${def.name}: write → ${res.status}`);
  }

  console.log('invalid rejects (one bad record per kind is refused):');
  for (const def of DEFINITIONS) {
    const c = invalid[def.name];
    if (c.by === 'gate') {
      const res = await createRecord(c.body);
      log(!res.ok && res.status === 422, `${def.name}: gate rejects ${reason(res)} → ${res.status}`);
    } else {
      const cv = clientViolations(def, c.body);
      log(cv.length > 0, `${def.name}: client rejects → ${cv.join('; ') || 'NOT CAUGHT'}`);
    }
  }

  console.log('dup-unique reject (client layer — gate does not enforce unique):');
  {
    const body = { ...valid.tenant, key: k('dup') };
    await createRecord(body); // first write takes the key
    const existing = await listRecords('tenant');
    const cv = clientViolations(tenant, { ...body }, existing);
    log(cv.some((v) => v.includes('unique')), `tenant: dup key → ${cv.join('; ') || 'NOT CAUGHT'}`);
  }

  console.log('device-limit reject (client layer — gate cannot express a count):');
  {
    const net = { content: { ...valid.network, max_devices: 2 } };
    const atCap = [{ content: {} }, { content: {} }]; // 2 meters == cap
    const v = deviceLimitViolation(net, atCap);
    log(Boolean(v), `meter on full network → ${v || 'NOT CAUGHT'}`);
    void meter; void network; // referenced for clarity that this is the meter/network rule
  }

  console.log(failures === 0 ? 'all checks passed' : `${failures} check(s) failed`);
  process.exit(failures === 0 ? 0 : 1);
}

function reason(res) {
  return res.body?.error ? `(${res.body.error.split(':').slice(-1)[0].trim()})` : '';
}

run().catch((err) => {
  console.error(err.message);
  process.exit(1);
});
