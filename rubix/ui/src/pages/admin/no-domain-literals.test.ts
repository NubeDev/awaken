// Guardrail: the admin console must stay domain-agnostic (SCOPE principle 4;
// ADMIN-UI "no screen hardcodes a domain type"). It reads only the substrate —
// records, kinds, tags, principals, capabilities — and must never bake in a
// domain vocabulary the way the operator UI (pages/Home, utils/derive) does.
//
// This scans the admin source for domain-term word literals. If a future edit
// reaches for `site`/`point`/`hvac`/`temp`/etc., this test fails — forcing the
// domain knowledge back into the data, where it belongs. The check is on whole
// words so substrate words that merely contain a fragment are not flagged.

import { readFileSync, readdirSync, statSync } from 'node:fs'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { describe, expect, it } from 'vitest'

// EMS/BMS domain vocabulary the operator UI uses — none of it belongs in admin.
const DOMAIN_TERMS = [
  'site',
  'sites',
  'point',
  'points',
  'equip',
  'equips',
  'equipment',
  'hvac',
  'zone',
  'zones',
  'setpoint',
  'damper',
  'reading',
  'readings',
  'building',
  'portfolio',
]

const here = fileURLToPath(new URL('.', import.meta.url))
// Scan the admin pages and the admin component dir.
const ADMIN_DIRS = [here, join(here, '..', '..', 'components', 'admin')]

function tsFiles(dir: string): string[] {
  const out: string[] = []
  for (const entry of readdirSync(dir)) {
    const full = join(dir, entry)
    if (statSync(full).isDirectory()) out.push(...tsFiles(full))
    else if (/\.tsx?$/.test(entry) && !entry.endsWith('.test.ts')) out.push(full)
  }
  return out
}

describe('admin console is domain-agnostic', () => {
  const files = ADMIN_DIRS.flatMap(tsFiles)

  it('scans at least the four screens', () => {
    expect(files.length).toBeGreaterThanOrEqual(4)
  })

  for (const file of ADMIN_DIRS.flatMap(tsFiles)) {
    it(`${file.split('/admin/').pop()} contains no domain vocabulary`, () => {
      const source = readFileSync(file, 'utf8')
      // Strip comments so an explanatory comment mentioning a domain term (e.g.
      // "a site, a task, and a device are all just records") doesn't trip the
      // guard — the rule is about CODE, not prose.
      const code = stripComments(source)
      const hits = DOMAIN_TERMS.filter((term) => new RegExp(`\\b${term}\\b`, 'i').test(code))
      expect(hits, `domain terms leaked into ${file}: ${hits.join(', ')}`).toEqual([])
    })
  }
})

/** Remove line and block comments so the scan only sees executable code. */
function stripComments(source: string): string {
  return source
    .replace(/\/\*[\s\S]*?\*\//g, '')
    .replace(/^\s*\/\/.*$/gm, '')
}
