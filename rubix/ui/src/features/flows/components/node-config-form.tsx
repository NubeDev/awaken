import { useState } from 'react'
import { Check, ChevronsUpDown } from 'lucide-react'
import type { ConfigFieldView } from '@/api/types'
import { useBoardOptions } from '@/api/hooks'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Switch } from '@/components/ui/switch'
import { Textarea } from '@/components/ui/textarea'
import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
} from '@/components/ui/command'
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'

/** The board's editing scope, needed to resolve `option_source` dropdowns. */
export type ConfigScope = { org?: string; site?: string }

type NodeConfigFormProps = {
  /** Schema fields for the selected node's component. */
  fields: ConfigFieldView[]
  /** Current config map of the node. */
  config: Record<string, unknown>
  /** Patch one config key; the editor merges and persists. */
  onChange: (key: string, value: unknown) => void
  /** Editing scope for `option_source` dropdowns (points, datasources, …). */
  scope?: ConfigScope
}

/**
 * Renders a node's config form from its component schema — no field is
 * hardcoded. Each `ConfigFieldView` picks its control by `field_type`; the
 * value is read from (and written back to) the node's `config` map. The
 * placeholder shows the schema default so an unset optional field reads as its
 * effective value without us silently writing it.
 */
export function NodeConfigForm({ fields, config, onChange, scope }: NodeConfigFormProps) {
  if (fields.length === 0) {
    return <p className='text-muted-foreground text-[11.5px]'>This node takes no config.</p>
  }
  return (
    <div className='flex flex-col gap-3'>
      {fields.map((field) => (
        <Field
          key={field.name}
          field={field}
          value={config[field.name]}
          config={config}
          onChange={onChange}
          scope={scope}
        />
      ))}
    </div>
  )
}

function Field({
  field,
  value,
  config,
  onChange,
  scope,
}: {
  field: ConfigFieldView
  value: unknown
  config: Record<string, unknown>
  onChange: (key: string, value: unknown) => void
  scope?: ConfigScope
}) {
  const defaultText = field.default !== undefined ? String(field.default) : undefined
  const id = `cfg-${field.name}`

  return (
    <div className='flex flex-col gap-1'>
      <div className='flex items-center justify-between'>
        <Label htmlFor={id} className='text-[11.5px] font-medium'>
          {field.label}
          {field.required && <span className='text-destructive ms-0.5'>*</span>}
        </Label>
        <span className='text-muted-foreground font-mono text-[9.5px]'>{field.field_type}</span>
      </div>

      <Control
        id={id}
        field={field}
        value={value}
        config={config}
        defaultText={defaultText}
        onChange={onChange}
        scope={scope}
      />

      {field.help && <p className='text-muted-foreground text-[10.5px]'>{field.help}</p>}
    </div>
  )
}

function Control({
  id,
  field,
  value,
  config,
  defaultText,
  onChange,
  scope,
}: {
  id: string
  field: ConfigFieldView
  value: unknown
  config: Record<string, unknown>
  defaultText?: string
  onChange: (key: string, value: unknown) => void
  scope?: ConfigScope
}) {
  // A backend-resolved dropdown takes priority over the field_type control: the
  // server decides the choices, the client just renders a searchable select.
  if (field.option_source) {
    return (
      <OptionSourceControl
        id={id}
        field={field}
        value={value}
        config={config}
        onChange={onChange}
        scope={scope}
      />
    )
  }

  if (field.field_type === 'boolean') {
    const on = value === undefined ? field.default === true : value === true
    return (
      <Switch id={id} checked={on} onCheckedChange={(checked) => onChange(field.name, checked)} />
    )
  }

  if (field.field_type === 'json') {
    return <JsonControl id={id} field={field} value={value} onChange={onChange} />
  }

  if (field.field_type === 'enum') {
    const current = value === undefined ? defaultText : String(value)
    return (
      <Select value={current} onValueChange={(v) => onChange(field.name, v)}>
        <SelectTrigger id={id} className='h-8 text-[12px]'>
          <SelectValue placeholder='Select…' />
        </SelectTrigger>
        <SelectContent>
          {(field.options ?? []).map((opt) => (
            <SelectItem key={opt} value={opt} className='text-[12px]'>
              {opt}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    )
  }

  const numeric = field.field_type === 'integer' || field.field_type === 'number'
  return (
    <Input
      id={id}
      type={numeric ? 'number' : 'text'}
      className='h-8 text-[12px]'
      value={value === undefined ? '' : String(value)}
      placeholder={defaultText}
      min={field.min}
      max={field.max}
      step={field.field_type === 'integer' ? 1 : 'any'}
      onChange={(e) => onChange(field.name, coerce(field, e.target.value))}
    />
  )
}

/**
 * A searchable dropdown for a field whose choices come from the backend
 * (`field.option_source`). The client is agnostic to what the source means: it
 * fetches `[{value,label}]` for the source in the board's scope and renders a
 * combobox. Options load lazily when the popover opens. The current value still
 * shows even if it is no longer in the list (a point since renamed/deleted), so
 * an edit never silently drops a saved id. `datasource_named` is narrowed by the
 * sibling `datasource` config value the same node already holds.
 */
function OptionSourceControl({
  id,
  field,
  value,
  config,
  onChange,
  scope,
}: {
  id: string
  field: ConfigFieldView
  value: unknown
  config: Record<string, unknown>
  onChange: (key: string, value: unknown) => void
  scope?: ConfigScope
}) {
  const [open, setOpen] = useState(false)
  const datasource =
    field.option_source === 'datasource_named' ? String(config.datasource ?? '') : undefined
  const { data, isLoading, isError } = useBoardOptions(
    field.option_source,
    { org: scope?.org, site: scope?.site, datasource: datasource || undefined },
    open
  )
  const options = data ?? []
  const current = value === undefined || value === '' ? '' : String(value)
  const selected = options.find((o) => o.value === current)
  // Show the saved value as its own row when the list hasn't loaded it (or no
  // longer contains it), so the selection is never visually lost.
  const display = selected?.label ?? (current || 'Select…')

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <Button
          id={id}
          type='button'
          variant='outline'
          role='combobox'
          aria-expanded={open}
          className='h-8 w-full justify-between text-[12px] font-normal'
        >
          <span className={cn('truncate', !current && 'text-muted-foreground')}>{display}</span>
          <ChevronsUpDown className='ms-2 size-3.5 shrink-0 opacity-50' />
        </Button>
      </PopoverTrigger>
      <PopoverContent className='w-(--radix-popover-trigger-width) p-0' align='start'>
        <Command>
          <CommandInput placeholder={`Search ${field.label.toLowerCase()}…`} className='text-[12px]' />
          <CommandList>
            <CommandEmpty className='text-muted-foreground px-3 py-3 text-[11.5px]'>
              {isLoading
                ? 'Loading…'
                : isError
                  ? 'Could not load options.'
                  : datasource === ''
                    ? 'Pick a datasource first.'
                    : 'No matches.'}
            </CommandEmpty>
            <CommandGroup>
              {options.map((opt) => (
                <CommandItem
                  key={opt.value}
                  value={`${opt.label} ${opt.value}`}
                  onSelect={() => {
                    onChange(field.name, opt.value === current ? undefined : opt.value)
                    setOpen(false)
                  }}
                  className='text-[12px]'
                >
                  <Check
                    className={cn(
                      'me-2 size-3.5',
                      opt.value === current ? 'opacity-100' : 'opacity-0'
                    )}
                  />
                  <span className='flex flex-col'>
                    <span>{opt.label}</span>
                    {opt.label !== opt.value && (
                      <span className='text-muted-foreground font-mono text-[10px]'>
                        {opt.value}
                      </span>
                    )}
                  </span>
                </CommandItem>
              ))}
            </CommandGroup>
          </CommandList>
        </Command>
      </PopoverContent>
    </Popover>
  )
}

/**
 * A JSON object control: a mono textarea that parses on edit and only writes a
 * valid object back to the config map. Invalid JSON shows an inline error and
 * does not persist — the last valid value is kept, so a half-typed edit never
 * corrupts the node config. An emptied field clears the key (schema default).
 */
function JsonControl({
  id,
  field,
  value,
  onChange,
}: {
  id: string
  field: ConfigFieldView
  value: unknown
  onChange: (key: string, value: unknown) => void
}) {
  const initial =
    value === undefined
      ? field.default !== undefined
        ? JSON.stringify(field.default, null, 2)
        : ''
      : JSON.stringify(value, null, 2)
  const [text, setText] = useState(initial)
  const [error, setError] = useState<string | undefined>()
  const errorId = `${id}-error`

  function handle(next: string) {
    setText(next)
    if (next.trim() === '') {
      setError(undefined)
      onChange(field.name, undefined)
      return
    }
    try {
      const parsed = JSON.parse(next)
      if (typeof parsed !== 'object' || parsed === null || Array.isArray(parsed)) {
        setError('Must be a JSON object (e.g. { "z": 3 }).')
        return
      }
      setError(undefined)
      onChange(field.name, parsed)
    } catch {
      setError('Invalid JSON.')
    }
  }

  return (
    <div className='flex flex-col gap-1'>
      <Textarea
        id={id}
        value={text}
        rows={4}
        spellCheck={false}
        aria-invalid={error ? true : undefined}
        aria-describedby={error ? errorId : undefined}
        className={`font-mono text-[11px] ${error ? 'border-destructive focus-visible:ring-destructive' : ''}`}
        placeholder='{ }'
        onChange={(e) => handle(e.target.value)}
      />
      {error && (
        <p id={errorId} className='text-destructive text-[10.5px]'>
          {error}
        </p>
      )}
    </div>
  )
}

/**
 * Coerce a text input back to the field's JSON type. An emptied field clears
 * the key (`undefined`) so it falls back to the schema default rather than
 * persisting an empty string the actor would reject.
 */
function coerce(field: ConfigFieldView, raw: string): unknown {
  if (raw === '') return undefined
  if (field.field_type === 'integer') {
    const n = Number.parseInt(raw, 10)
    return Number.isNaN(n) ? undefined : n
  }
  if (field.field_type === 'number') {
    const n = Number.parseFloat(raw)
    return Number.isNaN(n) ? undefined : n
  }
  return raw
}
