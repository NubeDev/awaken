// The variable bar above the board canvas (VARIABLES-AND-TEMPLATING.md §3) — a
// clean row of labelled controls, matching the old rubix bar: each visible
// variable is a single-select, a multi-select dropdown, or a textbox. A selection
// writes through to the URL (`?var-*`) so the board deep-links; the panels re-query
// off the resolved values. Hidden/constant variables resolve but are not shown.

import { ChevronDown } from 'lucide-react'
import type { BoardVariable, VariableScalar } from '../../api/boards'
import { Input } from '../ui/input'
import { Label } from '../ui/label'
import { Button } from '../ui/button'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '../ui/select'
import {
  DropdownMenu,
  DropdownMenuCheckboxItem,
  DropdownMenuContent,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '../ui/dropdown-menu'
import { ALL_SENTINEL, type Selection, type VariableOption } from './board-variables'

interface VariableBarProps {
  variables: BoardVariable[]
  options: Map<string, VariableOption[]>
  selections: Record<string, Selection>
  onChange: (name: string, selection: Selection) => void
}

export function VariableBar({ variables, options, selections, onChange }: VariableBarProps) {
  if (variables.length === 0) return null
  return (
    <div className="mb-4 flex flex-wrap items-end gap-3">
      {variables.map((v) => (
        <Field key={v.name} label={v.label ?? v.name}>
          <VariableControl
            variable={v}
            options={options.get(v.name) ?? []}
            selection={selections[v.name]}
            onChange={(sel) => onChange(v.name, sel)}
          />
        </Field>
      ))}
    </div>
  )
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="space-y-1">
      <Label className="text-[11px] text-muted-foreground">{label}</Label>
      {children}
    </div>
  )
}

interface ControlProps {
  variable: BoardVariable
  options: VariableOption[]
  selection: Selection
  onChange: (selection: Selection) => void
}

function VariableControl({ variable, options, selection, onChange }: ControlProps) {
  if (variable.kind === 'textbox') {
    return (
      <Input
        value={typeof selection === 'string' ? selection : ''}
        onChange={(e) => onChange(e.target.value)}
        placeholder={variable.name}
        className="h-8 w-44 text-[13px]"
      />
    )
  }

  if (variable.multi) {
    return (
      <MultiSelect
        variable={variable}
        options={options}
        selection={selection}
        onChange={onChange}
      />
    )
  }

  // Single-select. Radix Select keys on strings, so option scalars round-trip
  // through their string form.
  const current = Array.isArray(selection) ? selection[0] : selection
  return (
    <Select
      value={current === undefined ? undefined : String(current)}
      onValueChange={(s) => onChange(fromKey(s, options))}
    >
      <SelectTrigger className="h-8 w-44 text-[13px]">
        <SelectValue placeholder={`Select ${variable.label ?? variable.name}`} />
      </SelectTrigger>
      <SelectContent>
        {options.length === 0 ? (
          <div className="px-2 py-1.5 text-[12.5px] text-muted-foreground">No options</div>
        ) : (
          options.map((o) => (
            <SelectItem key={String(o.value)} value={String(o.value)}>
              {o.label}
            </SelectItem>
          ))
        )}
      </SelectContent>
    </Select>
  )
}

function MultiSelect({ variable, options, selection, onChange }: ControlProps) {
  const isAll = selection === ALL_SENTINEL
  const picked = new Set((Array.isArray(selection) ? selection : []).map(String))
  const summary = isAll
    ? 'All'
    : picked.size === 0
      ? 'None'
      : picked.size === 1
        ? (options.find((o) => picked.has(String(o.value)))?.label ?? '1 selected')
        : `${picked.size} selected`

  function toggle(value: VariableScalar) {
    const next = new Set(isAll ? options.map((o) => String(o.value)) : picked)
    const key = String(value)
    if (next.has(key)) next.delete(key)
    else next.add(key)
    onChange(options.filter((o) => next.has(String(o.value))).map((o) => o.value))
  }

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button
          variant="outline"
          size="sm"
          className="h-8 w-44 justify-between text-[13px] font-normal"
        >
          <span className="truncate">{summary}</span>
          <ChevronDown size={14} className="opacity-60" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="start" className="max-h-[280px] w-44 overflow-y-auto">
        <DropdownMenuLabel className="text-[11px]">
          {variable.label ?? variable.name}
        </DropdownMenuLabel>
        {variable.include_all && (
          <>
            <DropdownMenuCheckboxItem
              checked={isAll}
              onCheckedChange={() => onChange(ALL_SENTINEL)}
            >
              All
            </DropdownMenuCheckboxItem>
            <DropdownMenuSeparator />
          </>
        )}
        {options.map((o) => (
          <DropdownMenuCheckboxItem
            key={String(o.value)}
            checked={isAll || picked.has(String(o.value))}
            onCheckedChange={() => toggle(o.value)}
            onSelect={(e) => e.preventDefault()}
          >
            {o.label}
          </DropdownMenuCheckboxItem>
        ))}
      </DropdownMenuContent>
    </DropdownMenu>
  )
}

// Map a Radix string value back to its typed option scalar (preserving a numeric
// or boolean option's type).
function fromKey(key: string, options: VariableOption[]): VariableScalar {
  return options.find((o) => String(o.value) === key)?.value ?? key
}
