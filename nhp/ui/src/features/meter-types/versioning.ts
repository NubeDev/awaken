/**
 * Pure versioning helpers (DOMAIN-MODEL §versioning). Editing a meter-type bumps
 * `version` and leaves deployed meters untouched; a meter is "out of date" when
 * its stamped `meter_type_version` is below the type's current `version`. The
 * re-apply diff compares a meter's stamped registers against the type's current
 * `registers[]` so the admin sees what would change before confirming.
 */
import type { MeterRecord, RegisterDef, RegisterRec } from '@/api/records'

/** Deployment rollup for one meter-type: how many meters, how many out of date. */
export interface VersionRollup {
  total: number
  outOfDate: number
}

export function rollupForType(
  typeId: string,
  currentVersion: number,
  meters: MeterRecord[]
): VersionRollup {
  const mine = meters.filter((m) => m.content.meter_type === typeId)
  return {
    total: mine.length,
    outOfDate: mine.filter((m) => m.content.meter_type_version < currentVersion)
      .length,
  }
}

export type RegisterDiffKind = 'added' | 'removed' | 'changed' | 'unchanged'
export interface RegisterDiffRow {
  key: string
  name: string
  kind: RegisterDiffKind
}

/**
 * Diff a meter's current registers against the type's defs, matched by the def
 * `key` (the meter's register key is `${meterKey}--${defKey}`, WS-03 portfolio).
 */
export function diffRegisters(
  meterKey: string,
  typeDefs: RegisterDef[],
  meterRegisters: RegisterRec[]
): RegisterDiffRow[] {
  const prefix = `${meterKey}--`
  const stamped = new Map<string, RegisterRec>()
  for (const r of meterRegisters) {
    const defKey = r.content.key.startsWith(prefix)
      ? r.content.key.slice(prefix.length)
      : r.content.key
    stamped.set(defKey, r)
  }
  const rows: RegisterDiffRow[] = []
  const seen = new Set<string>()
  for (const def of typeDefs) {
    seen.add(def.key)
    const cur = stamped.get(def.key)
    if (!cur) {
      rows.push({ key: def.key, name: def.name, kind: 'added' })
      continue
    }
    const same = registerDefEqual(def, cur.content)
    rows.push({
      key: def.key,
      name: def.name,
      kind: same ? 'unchanged' : 'changed',
    })
  }
  for (const [defKey, reg] of stamped) {
    if (!seen.has(defKey)) {
      rows.push({ key: defKey, name: reg.content.name, kind: 'removed' })
    }
  }
  return rows
}

/** Field-by-field equality over the def's own keys (ignores meter/key/tags). */
function registerDefEqual(
  def: RegisterDef,
  reg: Partial<RegisterDef>
): boolean {
  const fields: (keyof RegisterDef)[] = [
    'name',
    'address',
    'fn_code',
    'datatype',
    'word_count',
    'byte_order',
    'scale',
    'offset',
    'signed',
    'unit',
    'quantity',
    'history',
    'chart_type',
    'chart_group',
    'precision',
  ]
  for (const f of fields) {
    if (JSON.stringify(def[f]) !== JSON.stringify(reg[f])) return false
  }
  return JSON.stringify(def.alarm ?? null) !== JSON.stringify(reg.alarm ?? null)
    ? false
    : true
}
