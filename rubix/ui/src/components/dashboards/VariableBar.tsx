// The variable bar above the board canvas (VARIABLES-AND-TEMPLATING.md §3). Each
// visible variable renders as a single-select, a multi-select dropdown, or a
// textbox; changing one writes the selection (the page mirrors it into `?var-*` and
// the board re-queries the dependent panels). A bound nav context shows as a small
// hint so it's clear *why* the board is scoped the way it is.

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
  /** Names bound by a nav node's context — shown as a hint, still editable. */
  boundByNav?: Set<string>
}

export function VariableBar({
  variables,
  options,
  selections,
  onChange,
  boundByNav,
}: VariableBarProps) {
  if (variables.length === 0) return null
  return (
    <div className="mb-3 flex flex-wrap items-end gap-3 rounded-lg border border-border bg-card/40 px-3 py-2">
      {variables.map((v) => (
        <div key={v.name} className="flex flex-col gap-1">
          <Label className="flex items-center gap-1.5 text-[11px] text-muted-foreground">
            {v.label ?? v.name}
            {boundByNav?.has(v.name) && (
              <span
                className="rounded bg-primary/10 px-1 text-[10px] font-medium text-primary"
                title="Bound by the navigation node opening this board"
              >
                nav
              </span>
            )}
          </Label>
          <VariableControl
            variable={v}
            options={options.get(v.name) ?? []}
            selection={selections[v.name]}
            onChange={(sel) => onChange(v.name, sel)}
          />
        </div>
      ))}
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
        className="h-8 w-[160px]"
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

  // Single-select. Radix Select needs string values, so option scalars are keyed
  // by their string form and mapped back on change.
  const current = Array.isArray(selection) ? selection[0] : selection
  return (
    <Select
      value={current === undefined ? undefined : String(current)}
      onValueChange={(s) => onChange(fromKey(s, options))}
    >
      <SelectTrigger className="h-8 w-[160px]">
        <SelectValue placeholder={`Select ${variable.label ?? variable.name}`} />
      </SelectTrigger>
      <SelectContent>
        {options.length === 0 ? (
          <SelectItem value="" disabled>
            No options
          </SelectItem>
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
        <Button variant="outline" size="sm" className="h-8 w-[160px] justify-between font-normal">
          <span className="truncate">{summary}</span>
          <ChevronDown size={14} className="opacity-60" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="start" className="max-h-[280px] w-[200px] overflow-y-auto">
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
            // Keep the menu open while picking several values.
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
