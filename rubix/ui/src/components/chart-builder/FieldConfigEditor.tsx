// FieldConfig defaults editor (§7) — authors the per-chart display defaults the
// §8 recharts widgets consume: the value-axis unit (from the unit registry) and
// fixed decimals. Compact by design: the defaults cover the common case; per-series
// byName/byRegex overrides, threshold ramps, and value mappings are richer surfaces
// the model already supports (field-config.ts) and can grow here later. Editing the
// unit also threads the column→quantity for backend conversion via the separate
// quantity picker (units carry their physical quantity, §2).

import type { FieldConfig } from './field-config'
import { UNIT_GROUPS } from './units'
import { Input } from '../ui/input'
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '../ui/select'

const NONE = '__none__'

interface FieldConfigEditorProps {
  value: FieldConfig | undefined
  onChange: (config: FieldConfig | undefined) => void
}

export function FieldConfigEditor({ value, onChange }: FieldConfigEditorProps) {
  const defaults = value?.defaults ?? {}

  // Merge a patch into defaults, pruning the whole config back to undefined when it
  // empties so an untouched chart serialises clean.
  const setDefaults = (patch: Partial<typeof defaults>) => {
    const next = { ...defaults, ...patch }
    for (const k of Object.keys(next) as (keyof typeof next)[]) {
      if (next[k] === undefined) delete next[k]
    }
    const config = Object.keys(next).length > 0 ? { ...value, defaults: next } : undefined
    onChange(config)
  }

  return (
    <div className="flex flex-wrap items-center gap-2">
      <span className="text-xs font-medium text-muted-foreground">Display</span>

      <Select
        value={defaults.unit ?? NONE}
        onValueChange={(v) => setDefaults({ unit: v === NONE ? undefined : v })}
      >
        <SelectTrigger className="h-8 w-[150px]">
          <SelectValue placeholder="Unit" />
        </SelectTrigger>
        <SelectContent>
          <SelectItem value={NONE}>No unit</SelectItem>
          {UNIT_GROUPS.map((g) => (
            <SelectGroup key={g.label}>
              <div className="px-2 py-1 text-[11px] font-medium uppercase tracking-wide text-muted-foreground">
                {g.label}
              </div>
              {g.units.map((u) => (
                <SelectItem key={u.id} value={u.id}>
                  {u.label}
                </SelectItem>
              ))}
            </SelectGroup>
          ))}
        </SelectContent>
      </Select>

      <Input
        type="number"
        min={0}
        max={10}
        placeholder="decimals"
        className="h-8 w-[110px]"
        value={defaults.decimals ?? ''}
        onChange={(e) =>
          setDefaults({ decimals: e.target.value === '' ? undefined : Number(e.target.value) })
        }
      />
    </div>
  )
}
