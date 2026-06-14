/**
 * Units & datetime preferences form (WS-11). Reads the caller's resolved prefs
 * and writes a `PreferencesPatch` per change via `PATCH /api/v1/me/preferences`;
 * the server re-resolves and the cache (and so the whole app's
 * `PreferencesProvider`) updates. Per-unit pickers are populated from the closed
 * registry (`GET /api/v1/units`), so they can never offer an unsupported unit.
 *
 * `unit_system` is the coarse toggle; the per-unit selects are fine overrides
 * (each offers `auto` = follow the system + an explicit unit). The look matches
 * the other settings panels — label + description + a right-aligned control.
 */
import { useMyPreferences, useUnits, useUpdateMyPreferences } from '@/api/hooks'
import type { PreferencesPatch, ResolvedPreferences } from '@/api/types'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'

/** One labelled control row, matching the settings panel rhythm. */
function Row({
  label,
  desc,
  children,
}: {
  label: string
  desc?: string
  children: React.ReactNode
}) {
  return (
    <div className='flex items-center justify-between gap-4 py-3'>
      <div className='space-y-0.5'>
        <p className='text-sm font-medium'>{label}</p>
        {desc ? <p className='text-xs text-muted-foreground'>{desc}</p> : null}
      </div>
      <div className='w-48 shrink-0'>{children}</div>
    </div>
  )
}

/** A select bound to one preference field; selecting emits a one-field patch. */
function PrefSelect({
  value,
  options,
  onChange,
  disabled,
}: {
  value: string
  options: { value: string; label: string }[]
  onChange: (value: string) => void
  disabled?: boolean
}) {
  return (
    <Select value={value} onValueChange={onChange} disabled={disabled}>
      <SelectTrigger className='w-full'>
        <SelectValue />
      </SelectTrigger>
      <SelectContent>
        {options.map((o) => (
          <SelectItem key={o.value} value={o.value}>
            {o.label}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  )
}

const UNIT_QUANTITY: Record<string, string> = {
  temperature_unit: 'temperature',
  pressure_unit: 'pressure',
  speed_unit: 'speed',
  length_unit: 'length',
  mass_unit: 'mass',
}

export function PreferencesForm() {
  const { data: prefs, isLoading } = useMyPreferences()
  const { data: units } = useUnits()
  const update = useUpdateMyPreferences()

  if (isLoading || !prefs) {
    return <p className='text-sm text-muted-foreground'>Loading preferences…</p>
  }

  const patch = (body: PreferencesPatch) => update.mutate(body)
  const disabled = update.isPending

  // Allowed units per quantity from the closed registry; `auto` always offered.
  const unitOptions = (field: keyof typeof UNIT_QUANTITY) => {
    const quantity = UNIT_QUANTITY[field]
    const entry = units?.quantities.find((q) => q.quantity === quantity)
    const allowed = entry?.allowed ?? []
    return [
      { value: 'auto', label: 'Auto (follow system)' },
      ...allowed.map((u) => ({ value: u, label: prettyUnit(u) })),
    ]
  }

  return (
    <div className='divide-y'>
      <Row label='Unit system' desc='The coarse metric/imperial default.'>
        <PrefSelect
          value={prefs.unit_system}
          disabled={disabled}
          options={[
            { value: 'metric', label: 'Metric' },
            { value: 'imperial', label: 'Imperial' },
          ]}
          onChange={(v) => patch({ unit_system: v as 'metric' | 'imperial' })}
        />
      </Row>

      {(Object.keys(UNIT_QUANTITY) as (keyof typeof UNIT_QUANTITY)[]).map(
        (field) => (
          <Row key={field} label={unitLabel(field)}>
            <PrefSelect
              value={prefs[field as keyof ResolvedPreferences] as string}
              disabled={disabled}
              options={unitOptions(field)}
              onChange={(v) => patch({ [field]: v } as PreferencesPatch)}
            />
          </Row>
        )
      )}

      <Row label='Date format'>
        <PrefSelect
          value={prefs.date_format}
          disabled={disabled}
          options={[
            { value: 'YYYY-MM-DD', label: 'YYYY-MM-DD' },
            { value: 'DD/MM/YYYY', label: 'DD/MM/YYYY' },
            { value: 'MM/DD/YYYY', label: 'MM/DD/YYYY' },
          ]}
          onChange={(v) => patch({ date_format: v })}
        />
      </Row>

      <Row label='Time format'>
        <PrefSelect
          value={prefs.time_format}
          disabled={disabled}
          options={[
            { value: '24h', label: '24-hour' },
            { value: '12h', label: '12-hour' },
          ]}
          onChange={(v) => patch({ time_format: v })}
        />
      </Row>

      <Row label='Week starts on'>
        <PrefSelect
          value={prefs.week_start}
          disabled={disabled}
          options={[
            { value: 'monday', label: 'Monday' },
            { value: 'sunday', label: 'Sunday' },
            { value: 'saturday', label: 'Saturday' },
          ]}
          onChange={(v) => patch({ week_start: v })}
        />
      </Row>

      <Row
        label='Number format'
        desc='Thousands/decimal separators for displayed numbers.'
      >
        <PrefSelect
          value={prefs.number_format}
          disabled={disabled}
          options={[
            { value: '1,234.56', label: '1,234.56' },
            { value: '1.234,56', label: '1.234,56' },
            { value: '1 234,56', label: '1 234,56' },
          ]}
          onChange={(v) => patch({ number_format: v })}
        />
      </Row>

      <Row label='Timezone' desc={`Currently ${prefs.timezone}.`}>
        <PrefSelect
          value={prefs.timezone}
          disabled={disabled}
          options={timezoneOptions(prefs.timezone)}
          onChange={(v) => patch({ timezone: v })}
        />
      </Row>

      <Row label='Locale' desc='Drives date/number formatting fallbacks.'>
        <PrefSelect
          value={prefs.locale}
          disabled={disabled}
          options={localeOptions(prefs.locale)}
          onChange={(v) => patch({ locale: v })}
        />
      </Row>
    </div>
  )
}

// --- small presentation helpers ----------------------------------------------

function unitLabel(field: string): string {
  return field
    .replace(/_unit$/, '')
    .replace(/^\w/, (c) => c.toUpperCase())
    .concat(' unit')
}

/** Turn a wire unit code (`meter_per_second`) into a readable label. */
function prettyUnit(code: string): string {
  return code.replace(/_/g, ' ')
}

/** A short, common IANA timezone list, always including the current value. */
function timezoneOptions(current: string) {
  const common = [
    'UTC',
    'Europe/London',
    'Europe/Paris',
    'America/New_York',
    'America/Chicago',
    'America/Los_Angeles',
    'Asia/Singapore',
    'Australia/Sydney',
  ]
  const set = current && !common.includes(current) ? [current, ...common] : common
  return set.map((tz) => ({ value: tz, label: tz }))
}

/** A short common-locale list, always including the current value. */
function localeOptions(current: string) {
  const common = ['en-US', 'en-GB', 'en-AU', 'fr-FR', 'de-DE', 'es-ES']
  const set =
    current && !common.includes(current) ? [current, ...common] : common
  return set.map((l) => ({ value: l, label: l }))
}
