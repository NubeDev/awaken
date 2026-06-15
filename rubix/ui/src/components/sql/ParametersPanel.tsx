// Query parameters panel — ported from Laminar's `parameters-panel.tsx` (§1a).
// Lists the editor's template parameters and lets you edit them; the values are
// substituted into `{{param}}` placeholders before the query runs (see
// applyParameters in sql-editor-store.ts). Laminar's `<DatePicker>` (a Radix
// popover + calendar) is replaced with a native datetime-local input to avoid
// pulling the calendar/popover dependency chain (§8) onto the spine.

import { Variable } from 'lucide-react'
import { format } from 'date-fns'

import type { SQLParameter } from './sql-editor-store'
import { Input } from '../ui/input'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '../ui/table'

interface ParametersPanelProps {
  parameters: SQLParameter[]
  onChange: (name: string, value: SQLParameter['value']) => void
}

export function ParametersPanel({ parameters, onChange }: ParametersPanelProps) {
  if (parameters.length === 0) {
    return (
      <div className="flex h-40 flex-col items-center justify-center gap-3 text-muted-foreground">
        <Variable className="size-7 opacity-50" />
        <p className="text-sm">No parameters configured.</p>
      </div>
    )
  }

  return (
    <div className="max-w-3xl rounded-xl border border-border bg-card/40">
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>Parameter</TableHead>
            <TableHead>Type</TableHead>
            <TableHead>Value</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {parameters.map((p) => (
            <TableRow key={p.name}>
              <TableCell>
                <code className="rounded bg-muted px-2 py-1 text-xs">{`{{${p.name}}}`}</code>
              </TableCell>
              <TableCell className="text-xs capitalize text-muted-foreground">{p.type}</TableCell>
              <TableCell className="w-72">
                <ParamInput parameter={p} onChange={onChange} />
              </TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </div>
  )
}

function ParamInput({
  parameter,
  onChange,
}: {
  parameter: SQLParameter
  onChange: ParametersPanelProps['onChange']
}) {
  switch (parameter.type) {
    case 'date':
      return (
        <Input
          type="datetime-local"
          className="h-8"
          value={parameter.value ? format(parameter.value, "yyyy-MM-dd'T'HH:mm") : ''}
          onChange={(e) => onChange(parameter.name, e.target.value ? new Date(e.target.value) : undefined)}
        />
      )
    case 'number':
      return (
        <Input
          type="number"
          className="h-8"
          value={parameter.value ?? ''}
          onChange={(e) => onChange(parameter.name, e.target.value === '' ? undefined : Number(e.target.value))}
        />
      )
    case 'string':
    default:
      return (
        <Input
          type="text"
          className="h-8"
          value={parameter.value ?? ''}
          onChange={(e) => onChange(parameter.name, e.target.value)}
        />
      )
  }
}
