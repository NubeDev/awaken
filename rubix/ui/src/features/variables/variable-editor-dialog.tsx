/**
 * The variable editor (docs/design/variables-and-templating.md §4): add / edit /
 * reorder / delete a dashboard's variables, choose a kind, author the option
 * query, preview resolved options, and set multi / include-all / hidden. Edits
 * are made on a working copy and persisted in one `PATCH /dashboards/{id}` (the
 * variables ride the dashboard snapshot — no separate audit path, per WS-08).
 */
import { useState } from 'react'
import { ArrowDown, ArrowUp, Plus, Trash2 } from 'lucide-react'
import { usePatchDashboard } from '@/api/hooks'
import type { Dashboard, Variable, VariableKind } from '@/api/types'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { useVariableResolution } from './use-resolution'
import {
  deleteVariable,
  moveVariable,
  newVariable,
  updateVariable,
} from './variable-editor'

const KINDS: VariableKind[] = [
  'site',
  'query',
  'custom',
  'constant',
  'interval',
  'textbox',
  'datasource',
]

type VariableEditorDialogProps = {
  open: boolean
  onOpenChange: (open: boolean) => void
  org: string | undefined
  dashboard: Dashboard
}

export function VariableEditorDialog(props: VariableEditorDialogProps) {
  const { open, onOpenChange } = props
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className='sm:max-w-2xl'>
        {open ? <Body {...props} /> : null}
      </DialogContent>
    </Dialog>
  )
}

function Body({ org, dashboard, onOpenChange }: VariableEditorDialogProps) {
  const patch = usePatchDashboard()
  const [variables, setVariables] = useState<Variable[]>(
    dashboard.variables ?? []
  )
  const [error, setError] = useState<string | null>(null)

  const save = () => {
    // Reject duplicate names client-side for a clear message; the server
    // re-validates authoritatively.
    const names = variables.map((v) => v.name.trim())
    if (names.some((n) => n === '')) {
      setError('Every variable needs a name.')
      return
    }
    if (new Set(names).size !== names.length) {
      setError('Variable names must be unique.')
      return
    }
    patch.mutate(
      { id: dashboard.id, body: { variables } },
      { onSuccess: () => onOpenChange(false), onError: (e) => setError((e as Error).message) }
    )
  }

  return (
    <>
      <DialogHeader>
        <DialogTitle>Variables — {dashboard.title}</DialogTitle>
        <DialogDescription>
          Parameterise this dashboard. Reference a variable in a tile's SQL as
          <span className='font-mono'> $name</span> or
          <span className='font-mono'> {'${name}'}</span>; values bind safely.
        </DialogDescription>
      </DialogHeader>

      <div className='max-h-[55vh] space-y-3 overflow-y-auto py-1'>
        {variables.length === 0 ? (
          <p className='text-[12px] text-muted-foreground'>
            No variables yet. Add a Site variable to drive a fleet of boards from
            one dashboard.
          </p>
        ) : (
          variables.map((variable, index) => (
            <VariableRow
              key={index}
              org={org}
              variables={variables}
              variable={variable}
              index={index}
              onChange={(patchObj) =>
                setVariables((vs) => updateVariable(vs, index, patchObj))
              }
              onDelete={() => setVariables((vs) => deleteVariable(vs, index))}
              onMove={(to) => setVariables((vs) => moveVariable(vs, index, to))}
            />
          ))
        )}

        <AddVariable
          onAdd={(kind) =>
            setVariables((vs) => [...vs, newVariable(kind, vs)])
          }
        />
      </div>

      {error ? <p className='text-[12px] text-sev-fault'>{error}</p> : null}

      <DialogFooter>
        <Button variant='ghost' onClick={() => onOpenChange(false)}>
          Cancel
        </Button>
        <Button onClick={save} disabled={patch.isPending}>
          {patch.isPending ? 'Saving…' : 'Save variables'}
        </Button>
      </DialogFooter>
    </>
  )
}

function AddVariable({ onAdd }: { onAdd: (kind: VariableKind) => void }) {
  const [kind, setKind] = useState<VariableKind>('site')
  return (
    <div className='flex items-center gap-2 border-t pt-3'>
      <Select value={kind} onValueChange={(v) => setKind(v as VariableKind)}>
        <SelectTrigger size='sm' className='w-40'>
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
      <Button size='sm' variant='outline' onClick={() => onAdd(kind)}>
        <Plus className='size-3.5' /> Add variable
      </Button>
    </div>
  )
}

function VariableRow({
  org,
  variables,
  variable,
  index,
  onChange,
  onDelete,
  onMove,
}: {
  org: string | undefined
  variables: Variable[]
  variable: Variable
  index: number
  onChange: (patch: Partial<Variable>) => void
  onDelete: () => void
  onMove: (to: number) => void
}) {
  return (
    <div className='space-y-2 rounded-md border p-2.5'>
      <div className='flex items-center gap-2'>
        <Input
          className='h-8 w-32 font-mono text-[12px]'
          value={variable.name}
          onChange={(e) => onChange({ name: e.target.value })}
          placeholder='name'
        />
        <Select
          value={variable.kind}
          onValueChange={(v) => onChange({ kind: v as VariableKind })}
        >
          <SelectTrigger size='sm' className='w-32'>
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
          className='h-8 flex-1 text-[12px]'
          value={variable.label ?? ''}
          onChange={(e) => onChange({ label: e.target.value })}
          placeholder='Label (optional)'
        />
        <Button
          size='icon'
          variant='ghost'
          className='size-7'
          disabled={index === 0}
          onClick={() => onMove(index - 1)}
          title='Move up'
        >
          <ArrowUp className='size-3.5' />
        </Button>
        <Button
          size='icon'
          variant='ghost'
          className='size-7'
          disabled={index === variables.length - 1}
          onClick={() => onMove(index + 1)}
          title='Move down'
        >
          <ArrowDown className='size-3.5' />
        </Button>
        <Button
          size='icon'
          variant='ghost'
          className='size-7 text-sev-fault'
          onClick={onDelete}
          title='Delete'
        >
          <Trash2 className='size-3.5' />
        </Button>
      </div>

      <KindConfig variable={variable} onChange={onChange} />

      <div className='flex flex-wrap items-center gap-4 text-[12px]'>
        <Toggle
          label='Multi'
          checked={!!variable.multi}
          onChange={(multi) => onChange({ multi })}
        />
        <Toggle
          label='Include All'
          checked={!!variable.include_all}
          onChange={(include_all) => onChange({ include_all })}
        />
        <Toggle
          label='Hidden'
          checked={!!variable.hidden}
          onChange={(hidden) => onChange({ hidden })}
        />
      </div>

      <OptionPreview org={org} variables={variables} variable={variable} />
    </div>
  )
}

function KindConfig({
  variable,
  onChange,
}: {
  variable: Variable
  onChange: (patch: Partial<Variable>) => void
}) {
  const { config } = variable
  if (config.kind === 'query') {
    return (
      <Input
        className='h-8 w-full font-mono text-[12px]'
        value={config.sql}
        onChange={(e) =>
          onChange({ config: { ...config, sql: e.target.value } })
        }
        placeholder="SELECT slug FROM sites WHERE site_id = '$parent'"
      />
    )
  }
  if (config.kind === 'custom' || config.kind === 'interval') {
    return (
      <Input
        className='h-8 w-full text-[12px]'
        value={config.options.join(', ')}
        onChange={(e) =>
          onChange({
            config: {
              ...config,
              options: e.target.value
                .split(',')
                .map((s) => s.trim())
                .filter(Boolean),
            },
          })
        }
        placeholder='Comma-separated options'
      />
    )
  }
  if (config.kind === 'constant') {
    return (
      <Input
        className='h-8 w-full text-[12px]'
        value={config.value === null ? '' : String(config.value)}
        onChange={(e) =>
          onChange({ config: { ...config, value: e.target.value } })
        }
        placeholder='Constant value'
      />
    )
  }
  return null
}

function OptionPreview({
  org,
  variables,
  variable,
}: {
  org: string | undefined
  variables: Variable[]
  variable: Variable
}) {
  // Resolve in the context of the full list so a query variable's parent
  // references resolve; show only this variable's options.
  const { resolved, error } = useVariableResolution({
    org,
    variables,
    selection: {},
  })
  if (error) return null
  const mine = resolved.find((r) => r.variable.name === variable.name)
  if (!mine || variable.config.kind === 'textbox') return null
  const preview = mine.options.slice(0, 8).map((o) => String(o))
  return (
    <p className='text-[11px] text-muted-foreground'>
      Options:{' '}
      {preview.length === 0 ? (
        <span className='italic'>none resolved</span>
      ) : (
        <span className='font-mono'>
          {preview.join(', ')}
          {mine.options.length > preview.length ? ' …' : ''}
        </span>
      )}
    </p>
  )
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
    <label className='flex items-center gap-1.5'>
      <input
        type='checkbox'
        checked={checked}
        onChange={(e) => onChange(e.target.checked)}
      />
      {label}
    </label>
  )
}
