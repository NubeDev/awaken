// Client-side enforcement of the rules rubix's gate does NOT enforce.
//
// Verified against the unmodified backend
// (rubix/crates/rubix-gate/src/command/validate.rs +
//  rubix/crates/rubix-core/src/collection/validate.rs): the gate enforces
// `required` and field TYPE only. It does NOT evaluate any `writeRule`, so closed
// enums are not enforced; and it explicitly does NOT check `unique`. For the POC
// those three rules live HERE (the registrar/UI layer) — the add-record path runs
// this before POSTing. Native enforcement is a logged RUBIX-TEAM item
// (nhp/docs/sessions/TODOs.md): when it lands this layer becomes belt-and-braces.
//
// Returns an array of human-readable violation strings (empty = passes).

import { ENUMS } from './enums.mjs';

// Check a candidate `content` against the closed-enum sets for its kind.
function enumViolations(kind, content) {
  const out = [];
  for (const [path, allowed] of Object.entries(ENUMS)) {
    const [k, field] = path.split('.');
    if (k !== kind) continue;
    const value = content[field];
    if (value !== undefined && value !== null && !allowed.includes(value)) {
      out.push(`${field}: \`${value}\` not in {${allowed.join(', ')}}`);
    }
  }
  return out;
}

// Check that `content`'s value for each `unique` field of `def` is not already
// taken among `existing` records of the same kind.
function uniqueViolations(def, content, existing) {
  const out = [];
  for (const field of def.schema.filter((f) => f.unique)) {
    const value = content[field.name];
    if (value === undefined || value === null) continue;
    const taken = existing.some((r) => r.content?.[field.name] === value);
    if (taken) out.push(`${field.name}: \`${value}\` is not unique`);
  }
  return out;
}

// Reject a meter create that would exceed its parent network's `max_devices`
// (DOMAIN-MODEL "Device limit"). rubix cannot express a count writeRule, so the
// cap is enforced here (and by the wizard, WS-06). `meters` is the set of meters
// already on `network`; `network.content.max_devices` is the cap.
export function deviceLimitViolation(network, meters) {
  const cap = network?.content?.max_devices;
  if (typeof cap !== 'number') return null;
  if (meters.length >= cap) {
    return `network \`${network.content.key}\` is at its device limit (${cap})`;
  }
  return null;
}

// All client-side violations for a candidate record of collection `def`.
export function clientViolations(def, content, existing = []) {
  return [
    ...enumViolations(def.name, content),
    ...uniqueViolations(def, content, existing),
  ];
}
