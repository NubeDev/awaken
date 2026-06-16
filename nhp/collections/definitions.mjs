// Barrel: the 7 NHP collection definitions in dependency (parent-before-child)
// order. Each lives in its own file (FILE-LAYOUT.md, one concept per file); this
// only re-exports. The order is the order the registrar upserts them so a
// relation's target collection is defined first.

import { tenant } from './tenant.mjs';
import { site } from './site.mjs';
import { gateway } from './gateway.mjs';
import { network } from './network.mjs';
import { meterType } from './meter-type.mjs';
import { meter } from './meter.mjs';
import { register } from './register.mjs';

export const DEFINITIONS = [
  tenant,
  site,
  gateway,
  network,
  meterType,
  meter,
  register,
];
