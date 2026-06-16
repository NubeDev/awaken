// The chart unit registry (DASHBOARDS-SCOPE.md §7) — display unit ids stored on a
// FieldConfig, each with a picker label and the symbol the value formatter
// appends. Adopted from nexus's `_shared/units.ts`, trimmed to the set Rubix
// ships, and — critically — a physical unit is tagged with the `rubix-prefs`
// `Quantity` it maps to (§2/§7) so units aren't just labels: the backend converts
// the column to the caller's unit system before it reaches the chart.
//
// The convertible PHYSICAL quantities (temperature/length/mass/speed) are the
// values the per-series quantity dropdown authors into the batch request's
// `quantities` map; the unit symbol is then a presentation-only label the
// formatter appends to the already-converted number.

/** A convertible physical quantity the backend can unit-convert (§2). The string
 *  values are exactly what `rubix-prefs::Quantity::parse` accepts and what the
 *  `POST /query` `quantities` map carries. */
export type PhysicalQuantity = 'temperature' | 'length' | 'mass' | 'speed'

/** A selectable display unit: the id stored on the field config, a picker label,
 *  the appended (or prepended) symbol, and — for physical units — the quantity it
 *  belongs to so the backend converts it. */
export interface UnitDef {
  id: string
  label: string
  /** The symbol appended/prepended to the value (empty → unitless). */
  symbol: string
  /** `true` → symbol leads the number ("$12"); default trails ("12 °C"). */
  prefix?: boolean
  /** A space between number and symbol (currencies/percent omit it). */
  space?: boolean
  /** The physical quantity this unit measures, if convertible by the backend. */
  quantity?: PhysicalQuantity
}

/** A named group of units for the picker's grouped dropdown. */
export interface UnitGroup {
  label: string
  units: ReadonlyArray<UnitDef>
}

export const UNIT_GROUPS: ReadonlyArray<UnitGroup> = [
  {
    label: 'Misc',
    units: [{ id: 'none', label: 'None', symbol: '' }],
  },
  {
    label: 'Percentage',
    units: [
      { id: 'percent', label: 'Percent (0–100)', symbol: '%' },
      { id: 'percentunit', label: 'Percent (0.0–1.0)', symbol: '%' },
    ],
  },
  {
    label: 'Temperature',
    units: [
      { id: 'celsius', label: 'Celsius', symbol: '°C', space: true, quantity: 'temperature' },
      { id: 'fahrenheit', label: 'Fahrenheit', symbol: '°F', space: true, quantity: 'temperature' },
    ],
  },
  {
    label: 'Length',
    units: [
      { id: 'meter', label: 'Metre', symbol: 'm', space: true, quantity: 'length' },
      { id: 'foot', label: 'Foot', symbol: 'ft', space: true, quantity: 'length' },
    ],
  },
  {
    label: 'Mass',
    units: [
      { id: 'kilogram', label: 'Kilogram', symbol: 'kg', space: true, quantity: 'mass' },
      { id: 'pound', label: 'Pound', symbol: 'lb', space: true, quantity: 'mass' },
    ],
  },
  {
    label: 'Speed',
    units: [
      { id: 'mps', label: 'Metres / sec', symbol: 'm/s', space: true, quantity: 'speed' },
      { id: 'mph', label: 'Miles / hour', symbol: 'mph', space: true, quantity: 'speed' },
    ],
  },
  {
    label: 'Energy & power',
    units: [
      { id: 'watt', label: 'Watt', symbol: 'W', space: true },
      { id: 'kilowatt', label: 'Kilowatt', symbol: 'kW', space: true },
      { id: 'watthour', label: 'Watt-hour', symbol: 'Wh', space: true },
      { id: 'kilowatthour', label: 'Kilowatt-hour', symbol: 'kWh', space: true },
    ],
  },
  {
    label: 'Currency',
    units: [
      { id: 'usd', label: 'US Dollar', symbol: '$', prefix: true },
      { id: 'eur', label: 'Euro', symbol: '€', prefix: true },
      { id: 'gbp', label: 'Pound', symbol: '£', prefix: true },
    ],
  },
]

// O(1) lookup of a unit by its stored id.
const UNIT_BY_ID: ReadonlyMap<string, UnitDef> = new Map(
  UNIT_GROUPS.flatMap((g) => g.units).map((u) => [u.id, u]),
)

/** The unit definition for `id`, or undefined for an unknown/stale id (so the
 *  formatter degrades to unitless rather than throwing). */
export function unitDef(id: string | undefined): UnitDef | undefined {
  return id == null ? undefined : UNIT_BY_ID.get(id)
}

/** The physical quantity a unit id belongs to, if the backend can convert it. */
export function quantityOf(unitId: string | undefined): PhysicalQuantity | undefined {
  return unitDef(unitId)?.quantity
}

/** The convertible quantities offered in the per-series quantity dropdown — the
 *  values threaded into the batch request's `quantities` map (§2/§7). */
export const PHYSICAL_QUANTITIES: ReadonlyArray<{ value: PhysicalQuantity; label: string }> = [
  { value: 'temperature', label: 'Temperature' },
  { value: 'length', label: 'Length' },
  { value: 'mass', label: 'Mass' },
  { value: 'speed', label: 'Speed' },
]
