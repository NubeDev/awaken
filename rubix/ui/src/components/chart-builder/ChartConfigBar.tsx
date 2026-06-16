// The chart builder's structural controls: type, x/y, breakdown (multi-series
// split), display mode (headline total/average), and the Y column's physical
// quantity. Extracted from the Query console so the SAME settings drive both the
// console and the in-dashboard "edit chart" dialog — a chart authored one place is
// editable the other with identical controls. Operates purely on `ChartConfig`.

import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '../ui/select'
import { ChartType, type ChartConfig, type DisplayMode } from './types'
import { PHYSICAL_QUANTITIES, type PhysicalQuantity } from './units'
import { WIDGETS, allowsBreakdown, needsX, needsY } from './catalog'
import type { ColumnInfo } from './utils'

const NONE = '__none__'

export function ChartConfigBar({
  columns,
  config,
  onChange,
}: {
  columns: ColumnInfo[]
  config: ChartConfig
  onChange: (config: ChartConfig) => void
}) {
  const set = (patch: Partial<ChartConfig>) => onChange({ ...config, ...patch } as ChartConfig)
  // Which pickers to show is the catalog's call (§8 roles), not a per-type rule
  // here — so a new widget's fields follow from its descriptor. This is what stops
  // a Pie offering a Breakdown it can't use.
  const showX = needsX(config.type)
  const showY = needsY(config.type)
  const showBreakdown = allowsBreakdown(config.type) && config.type !== ChartType.Table
  return (
    <div className="flex flex-wrap items-center gap-2">
      <Select value={config.type ?? ChartType.LineChart} onValueChange={(v) => set({ type: v as ChartType })}>
        <SelectTrigger className="w-[140px]">
          <SelectValue placeholder="Chart type" />
        </SelectTrigger>
        <SelectContent>
          {/* Driven by the widget catalog (§8) so a new widget is one entry. */}
          {WIDGETS.map((w) => (
            <SelectItem key={w.type} value={w.type}>
              {w.label}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
      {showX && <ColumnPicker label="X" value={config.x} columns={columns} onChange={(x) => set({ x })} />}
      {showY && <ColumnPicker label="Y" value={config.y} columns={columns} onChange={(y) => set({ y })} />}
      {showBreakdown && (
        <>
          <ColumnPicker
            label="Breakdown"
            value={config.breakdown}
            columns={columns}
            allowNone
            onChange={(b) => set({ breakdown: b === NONE ? undefined : b })}
          />
          <Select
            value={config.displayMode ?? 'none'}
            onValueChange={(v) => set({ displayMode: v as DisplayMode })}
          >
            <SelectTrigger className="w-[140px]">
              <SelectValue placeholder="Display" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="none">No headline</SelectItem>
              <SelectItem value="total">Total</SelectItem>
              <SelectItem value="average">Average</SelectItem>
            </SelectContent>
          </Select>
        </>
      )}
      {/* Per-series quantity (§7): author the Y column's physical quantity so the
          query API converts it to the caller's unit system (§2). Only for widgets
          that have a value column. */}
      {showY && config.y && (
        <QuantityPicker
          column={config.y}
          value={config.quantities?.[config.y]}
          onChange={(q) => set({ quantities: setQuantity(config.quantities, config.y!, q) })}
        />
      )}
    </div>
  )
}

// Set (or clear, when `q` is undefined) the quantity for `column` in the map,
// returning undefined when the map empties so an untouched chart serialises clean.
export function setQuantity(
  map: Record<string, PhysicalQuantity> | undefined,
  column: string,
  q: PhysicalQuantity | undefined,
): Record<string, PhysicalQuantity> | undefined {
  const next = { ...(map ?? {}) }
  if (q) next[column] = q
  else delete next[column]
  return Object.keys(next).length > 0 ? next : undefined
}

// The Y column's physical quantity — drives backend unit conversion (§2/§7).
function QuantityPicker({
  column,
  value,
  onChange,
}: {
  column: string
  value?: PhysicalQuantity
  onChange: (q: PhysicalQuantity | undefined) => void
}) {
  return (
    <Select
      value={value ?? NONE}
      onValueChange={(v) => onChange(v === NONE ? undefined : (v as PhysicalQuantity))}
    >
      <SelectTrigger className="w-[150px]">
        <SelectValue placeholder={`${column} quantity`} />
      </SelectTrigger>
      <SelectContent>
        <SelectItem value={NONE}>No quantity</SelectItem>
        {PHYSICAL_QUANTITIES.map((q) => (
          <SelectItem key={q.value} value={q.value}>
            {q.label}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  )
}

function ColumnPicker({
  label,
  value,
  columns,
  allowNone,
  onChange,
}: {
  label: string
  value?: string
  columns: { name: string }[]
  allowNone?: boolean
  onChange: (value: string) => void
}) {
  return (
    <Select value={value ?? (allowNone ? NONE : '')} onValueChange={onChange}>
      <SelectTrigger className="w-[150px]">
        <SelectValue placeholder={label} />
      </SelectTrigger>
      <SelectContent>
        {allowNone && <SelectItem value={NONE}>No {label.toLowerCase()}</SelectItem>}
        {columns
          .filter((c) => c.name !== '')
          .map((c) => (
            <SelectItem key={c.name} value={c.name}>
              {c.name}
            </SelectItem>
          ))}
      </SelectContent>
    </Select>
  )
}
