// The variable editor (VARIABLES-AND-TEMPLATING.md §4): add / edit / reorder /
// delete a board's variables, choose a kind, author the option source, set
// multi / include-all / hidden, and a default value. Edits are made on a working
// copy and persisted in one save (the variables ride the board record's content,
// so it is one audited gate write — no separate path).

import { useState } from 'react'
import { ArrowDown, ArrowUp, Plus, Trash2 } from 'lucide-react'
import type { BoardVariable, VariableKind } from '../../api/boards'
import { Button } from '../ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '../ui/dialog'
import { Input } from '../ui/input'
import { Checkbox } from '../ui/checkbox'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '../ui/select'
import { deleteVariable, moveVariable, newVariable, updateVariable } from './variable-editor'

const KINDS: VariableKind[] = ['site', 'query', 'custom', 'constant', 'textbox']

const KIND_HINT: Record<VariableKind, string> = {
  site: "Options are the tenant's sites; the value is the site key.",
  query: 'Options come from the first column of a SQL query.',
  custom: 'A fixed, comma-separated option list.',
  constant: 'A single fixed value (usually hidden).',
  textbox: 'Free text typed by the viewer.',
}

interface VariableEditorDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  boardName: string
  variables: BoardVariable[]
  onSave: (variables: BoardVariable[]) => void
  saving?: boolean
}

export function VariableEditorDialog(props: VariableEditorDialogProps) {
  return (
    <Dialog open={props.open} onOpenChange={props.onOpenChange}>
      <DialogContent className="sm:max-w-2xl">
        {props.open ? <Body {...props} /> : null}
      </DialogContent>
    </Dialog>
  )
}

function Body({
  boardName,
  variables: initial,
  onSave,
  onOpenChange,
  saving,
}: VariableEditorDialogProps) {
  const [variables, setVariables] = useState<BoardVariable[]>(initial)
  const [error, setError] = useState<string | null>(null)

  function save() {
    const names = variables.map((v) => v.name.trim())
    if (names.some((n) => n === '')) return setError('Every variable needs a name.')
    if (new Set(names).size !== names.length) return setError('Variable names must be unique.')
    setError(null)
    onSave(variables.map((v) => ({ ...v, name: v.name.trim() })))
  }

  return (
    <>
      <DialogHeader>
        <DialogTitle>Variables — {boardName}</DialogTitle>
        <DialogDescription>
          Parameterise this board. Reference a variable in a chart's SQL as{' '}
          <span className="mono">$name</span> or <span className="mono">{'$__sqlIn(name)'}</span>;
          values bind safely server-side.
        </DialogDescription>
      </DialogHeader>

      <div className="max-h-[55vh] space-y-3 overflow-y-auto py-1">
        {variables.length === 0 ? (
          <p className="text-[12px] text-muted-foreground">
            No variables yet. Add a <span className="mono">site</span> variable to drive a fleet of
            boards from one.
          </p>
        ) : (
          variables.map((variable, index) => (
            <VariableRow
              key={index}
              variables={variables}
              variable={variable}
              index={index}
              onChange={(patch) => setVariables((vs) => updateVariable(vs, index, patch))}
              onDelete={() => setVariables((vs) => deleteVariable(vs, index))}
              onMove={(to) => setVariables((vs) => moveVariable(vs, index, to))}
            />
          ))
        )}
        <AddVariable onAdd={(kind) => setVariables((vs) => [...vs, newVariable(kind, vs)])} />
      </div>

      {error ? <p className="text-[12px] text-destructive">{error}</p> : null}

      <DialogFooter>
        <Button variant="ghost" onClick={() => onOpenChange(false)}>
          Cancel
        </Button>
        <Button onClick={save} disabled={saving}>
          {saving ? 'Saving…' : 'Save variables'}
        </Button>
      </DialogFooter>
    </>
  )
}

function AddVariable({ onAdd }: { onAdd: (kind: VariableKind) => void }) {
  const [kind, setKind] = useState<VariableKind>('site')
  return (
    <div className="flex items-center gap-2 border-t border-border pt-3">
      <Select value={kind} onValueChange={(v) => setKind(v as VariableKind)}>
        <SelectTrigger className="h-8 w-40 text-[13px]">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          {KINDS.map((k) => (
            <SelectItem key={k} value={k}>
              {k}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
      <Button size="sm" variant="outline" onClick={() => onAdd(kind)} className="gap-1.5">
        <Plus className="size-3.5" /> Add variable
      </Button>
    </div>
  )
}

interface RowProps {
  variables: BoardVariable[]
  variable: BoardVariable
  index: number
  onChange: (patch: Partial<BoardVariable>) => void
  onDelete: () => void
  onMove: (to: number) => void
}

function VariableRow({ variables, variable, index, onChange, onDelete, onMove }: RowProps) {
  return (
    <div className="space-y-2 rounded-md border border-border p-2.5">
      <div className="flex items-center gap-2">
        <Input
          className="mono h-8 w-32 text-[12px]"
          value={variable.name}
          onChange={(e) => onChange({ name: e.target.value })}
          placeholder="name"
        />
        <Select value={variable.kind} onValueChange={(v) => onChange({ kind: v as VariableKind })}>
          <SelectTrigger className="h-8 w-32 text-[13px]">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {KINDS.map((k) => (
              <SelectItem key={k} value={k}>
                {k}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        <Input
          className="h-8 flex-1 text-[12px]"
          value={variable.label ?? ''}
          onChange={(e) => onChange({ label: e.target.value })}
          placeholder="Label (optional)"
        />
        <Button
          size="icon"
          variant="ghost"
          className="size-7"
          disabled={index === 0}
          onClick={() => onMove(index - 1)}
          title="Move up"
        >
          <ArrowUp className="size-3.5" />
        </Button>
        <Button
          size="icon"
          variant="ghost"
          className="size-7"
          disabled={index === variables.length - 1}
          onClick={() => onMove(index + 1)}
          title="Move down"
        >
          <ArrowDown className="size-3.5" />
        </Button>
        <Button
          size="icon"
          variant="ghost"
          className="size-7 text-muted-foreground hover:text-destructive"
          onClick={onDelete}
          title="Delete"
        >
          <Trash2 className="size-3.5" />
        </Button>
      </div>

      <KindConfig variable={variable} onChange={onChange} />
      <p className="text-[11px] text-muted-foreground">{KIND_HINT[variable.kind]}</p>

      <div className="flex flex-wrap items-center gap-4 text-[12px]">
        <Toggle
          label="Multi"
          checked={!!variable.multi}
          onChange={(multi) => onChange({ multi })}
        />
        <Toggle
          label="Include All"
          checked={!!variable.include_all}
          onChange={(include_all) => onChange({ include_all })}
        />
        <Toggle
          label="Hidden"
          checked={!!variable.hidden}
          onChange={(hidden) => onChange({ hidden })}
        />
      </div>
    </div>
  )
}

function KindConfig({
  variable,
  onChange,
}: {
  variable: BoardVariable
  onChange: (patch: Partial<BoardVariable>) => void
}) {
  if (variable.kind === 'query') {
    return (
      <Input
        className="mono h-8 w-full text-[12px]"
        value={variable.config?.query ?? ''}
        onChange={(e) => onChange({ config: { ...variable.config, query: e.target.value } })}
        placeholder="SELECT json_get(json_get(content,'content'),'key') AS k FROM record WHERE …"
      />
    )
  }
  if (variable.kind === 'custom') {
    return (
      <Input
        className="h-8 w-full text-[12px]"
        value={(variable.config?.options ?? []).join(', ')}
        onChange={(e) =>
          onChange({
            config: {
              ...variable.config,
              options: e.target.value
                .split(',')
                .map((s) => s.trim())
                .filter(Boolean),
            },
          })
        }
        placeholder="Comma-separated options"
      />
    )
  }
  if (variable.kind === 'constant' || variable.kind === 'textbox') {
    return (
      <Input
        className="h-8 w-full text-[12px]"
        value={typeof variable.current === 'string' ? variable.current : ''}
        onChange={(e) => onChange({ current: e.target.value })}
        placeholder={variable.kind === 'constant' ? 'Constant value' : 'Default text (optional)'}
      />
    )
  }
  return null
}

function Toggle({
  label,
  checked,
  onChange,
}: {
  label: string
  checked: boolean
  onChange: (checked: boolean) => void
}) {
  return (
    <label className="flex cursor-pointer items-center gap-1.5">
      <Checkbox checked={checked} onCheckedChange={(c) => onChange(c === true)} />
      {label}
    </label>
  )
}
