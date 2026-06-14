/**
 * The variable bar (docs/design/variables-and-templating.md §3): renders each
 * visible dashboard variable as a single/multi-select or textbox, with an "All"
 * option when `include_all`. A selection writes through to the URL (`?var-*`) so
 * it deep-links; widgets re-query off the resolved values. Hidden variables
 * (e.g. a `constant` used only inside another variable's SQL) are not rendered.
 */
import type { ResolvedVariable } from './use-resolution'
import type { VariableValue } from '@/api/types'
import { Label } from '@/components/ui/label'
import { Input } from '@/components/ui/input'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'

const ALL = '__all__'

type VariableBarProps = {
  resolved: ResolvedVariable[]
  error?: Error
  onChange: (name: string, value: VariableValue) => void
}

export function VariableBar({ resolved, error, onChange }: VariableBarProps) {
  const visible = resolved.filter((r) => !r.variable.hidden)
  if (error) {
    return (
      <div className='rounded-md border border-sev-fault/40 bg-sev-fault/5 px-3 py-2 text-[12px] text-sev-fault'>
        {error.message}
      </div>
    )
  }
  if (visible.length === 0) return null

  return (
    <div className='flex flex-wrap items-end gap-3'>
      {visible.map((r) => (
        <VariableControl key={r.variable.name} resolved={r} onChange={onChange} />
      ))}
    </div>
  )
}

function VariableControl({
  resolved,
  onChange,
}: {
  resolved: ResolvedVariable
  onChange: (name: string, value: VariableValue) => void
}) {
  const { variable, options, current } = resolved
  const label = variable.label ?? variable.name

  if (variable.config.kind === 'textbox') {
    return (
      <Field label={label}>
        <Input
          className='h-8 w-44 text-[12px]'
          value={typeof current === 'string' ? current : ''}
          onChange={(e) => onChange(variable.name, e.target.value)}
          placeholder={label}
        />
      </Field>
    )
  }

  // A single-select (multi is expressed via "All" → the full option list; an
  // explicit per-value multi picker is a later enhancement, see the editor).
  const selected =
    variable.multi && Array.isArray(current)
      ? ALL
      : current === null || current === undefined
        ? ''
        : String(current)

  const onValueChange = (value: string) => {
    if (value === ALL) {
      onChange(variable.name, options.slice())
    } else {
      onChange(variable.name, value)
    }
  }

  return (
    <Field label={label}>
      <Select value={selected} onValueChange={onValueChange}>
        <SelectTrigger size='sm' className='w-44'>
          <SelectValue placeholder={`Select ${label}`} />
        </SelectTrigger>
        <SelectContent>
          {variable.include_all ? (
            <SelectItem value={ALL}>All</SelectItem>
          ) : null}
          {options.map((opt) => {
            const v = opt === null ? '' : String(opt)
            return (
              <SelectItem key={v} value={v}>
                {v}
              </SelectItem>
            )
          })}
        </SelectContent>
      </Select>
    </Field>
  )
}

function Field({
  label,
  children,
}: {
  label: string
  children: React.ReactNode
}) {
  return (
    <div className='space-y-1'>
      <Label className='text-[11px] text-muted-foreground'>{label}</Label>
      {children}
    </div>
  )
}
