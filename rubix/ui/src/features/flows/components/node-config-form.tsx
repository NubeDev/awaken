import { useState } from 'react'
import type { ConfigFieldView } from '@/api/types'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Switch } from '@/components/ui/switch'
import { Textarea } from '@/components/ui/textarea'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'

type NodeConfigFormProps = {
  /** Schema fields for the selected node's component. */
  fields: ConfigFieldView[]
  /** Current config map of the node. */
  config: Record<string, unknown>
  /** Patch one config key; the editor merges and persists. */
  onChange: (key: string, value: unknown) => void
}

/**
 * Renders a node's config form from its component schema — no field is
 * hardcoded. Each `ConfigFieldView` picks its control by `field_type`; the
 * value is read from (and written back to) the node's `config` map. The
 * placeholder shows the schema default so an unset optional field reads as its
 * effective value without us silently writing it.
 */
export function NodeConfigForm({ fields, config, onChange }: NodeConfigFormProps) {
  if (fields.length === 0) {
    return <p className='text-muted-foreground text-[11.5px]'>This node takes no config.</p>
  }
  return (
    <div className='flex flex-col gap-3'>
      {fields.map((field) => (
        <Field key={field.name} field={field} value={config[field.name]} onChange={onChange} />
      ))}
    </div>
  )
}

function Field({
  field,
  value,
  onChange,
}: {
  field: ConfigFieldView
  value: unknown
  onChange: (key: string, value: unknown) => void
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

      <Control id={id} field={field} value={value} defaultText={defaultText} onChange={onChange} />

      {field.help && <p className='text-muted-foreground text-[10.5px]'>{field.help}</p>}
    </div>
  )
}

function Control({
  id,
  field,
  value,
  defaultText,
  onChange,
}: {
  id: string
  field: ConfigFieldView
  value: unknown
  defaultText?: string
  onChange: (key: string, value: unknown) => void
}) {
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
