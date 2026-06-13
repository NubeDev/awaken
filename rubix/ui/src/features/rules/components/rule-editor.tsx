import { useEffect, useMemo, useState } from 'react'
import { Plus, Save, Trash2, X } from 'lucide-react'
import { toast } from 'sonner'
import { ApiError } from '@/api/client'
import {
  useCreateRule,
  useDeleteRule,
  useReferencingRules,
  useUpdateRule,
} from '@/api/hooks'
import type { ParamSchema, RuleView } from '@/api/types'
import { Button } from '@/components/ui/button'
import { Card } from '@/components/ui/card'
import { ConfirmDialog } from '@/components/confirm-dialog'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Switch } from '@/components/ui/switch'
import { CodeEditor } from '@/components/code/code-editor'
import { ChangeImpact } from './change-impact'

type ParamRow = { key: string; required: boolean; description: string }

function schemaToRows(schema: ParamSchema | undefined): ParamRow[] {
  if (!schema) return []
  return Object.entries(schema.params).map(([key, spec]) => ({
    key,
    required: spec.required,
    description: spec.description ?? '',
  }))
}

function rowsToSchema(rows: ParamRow[]): ParamSchema {
  const params: ParamSchema['params'] = {}
  for (const r of rows) {
    if (!r.key.trim()) continue
    params[r.key.trim()] = {
      required: r.required,
      ...(r.description.trim() ? { description: r.description.trim() } : {}),
    }
  }
  return { params }
}

type RuleEditorProps = {
  org: string | undefined
  /** The selected rule, or `null` for the "new rule" draft. */
  rule: RuleView | null
  /** Lift the live draft so the debugger can dry-run the unsaved script. */
  onDraftChange: (draft: { script: string; params: Record<string, unknown> }) => void
  onSaved: (name: string) => void
  onDeleted: () => void
}

const NEW_SCRIPT_TEMPLATE = `// A rule operates on \`df\` (ts + value columns) and returns a verdict.
let z = df.zscore("value");
if z.anomalies("value", 3.0).describe().any_true("value") {
  finding("warning", "anomaly detected")
} else {
  clear()
}
`

/**
 * The rule editor: name + params schema + the Rhai script in CodeMirror, with
 * inline 409 handling and the change-impact (referencing) safety surface shown
 * before save/delete. Save is create (new) or PUT (existing). The live script +
 * params lift to the parent so the debugger dry-runs exactly what is on screen.
 */
export function RuleEditor({
  org,
  rule,
  onDraftChange,
  onSaved,
  onDeleted,
}: RuleEditorProps) {
  const isNew = rule === null
  const [name, setName] = useState(rule?.name ?? '')
  const [script, setScript] = useState(rule?.script ?? NEW_SCRIPT_TEMPLATE)
  const [paramRows, setParamRows] = useState<ParamRow[]>(schemaToRows(rule?.params))
  const [nameError, setNameError] = useState<string | undefined>()
  const [confirmDelete, setConfirmDelete] = useState(false)

  const create = useCreateRule(org)
  const update = useUpdateRule(org)
  const del = useDeleteRule(org)
  const { data: referencing = [] } = useReferencingRules(
    org,
    isNew ? undefined : rule?.name
  )

  // No reset effect: the parent gives this component a `key` per rule (and a
  // distinct key for the "new" draft), so a selection change remounts it and the
  // `useState` initializers above seed from the freshly-selected rule.

  // Lift the draft so the debugger can dry-run the unsaved script.
  const paramObject = useMemo(() => {
    const obj: Record<string, unknown> = {}
    for (const r of paramRows) if (r.key.trim()) obj[r.key.trim()] = ''
    return obj
  }, [paramRows])
  useEffect(() => {
    onDraftChange({ script, params: paramObject })
  }, [script, paramObject, onDraftChange])

  const paramNames = useMemo(
    () => paramRows.map((r) => r.key.trim()).filter(Boolean),
    [paramRows]
  )

  const save = () => {
    setNameError(undefined)
    const schema = rowsToSchema(paramRows)
    if (isNew) {
      const trimmed = name.trim()
      if (!/^[a-z0-9-]+$/.test(trimmed)) {
        setNameError('Use a lowercase slug (a–z, 0–9, hyphen).')
        return
      }
      create.mutate(
        { name: trimmed, script, params: schema },
        {
          onSuccess: () => {
            toast.success(`Rule "${trimmed}" created`)
            onSaved(trimmed)
          },
          onError: (e) => {
            if (e instanceof ApiError && e.status === 409) {
              setNameError('A rule with this name already exists in this org.')
            } else {
              toast.error(e instanceof ApiError ? e.message : 'Create failed')
            }
          },
        }
      )
      return
    }
    update.mutate(
      { name: rule!.name, body: { script, params: schema } },
      {
        onSuccess: () => {
          toast.success(`Rule "${rule!.name}" saved`)
          onSaved(rule!.name)
        },
        onError: (e) =>
          toast.error(e instanceof ApiError ? e.message : 'Save failed'),
      }
    )
  }

  const saving = create.isPending || update.isPending

  return (
    <Card className='gap-3 p-4'>
      <div className='flex flex-wrap items-end gap-2'>
        <div className='flex min-w-[200px] flex-1 flex-col gap-1'>
          <Label htmlFor='rule-name' className='text-[11px]'>
            Name
          </Label>
          <Input
            id='rule-name'
            value={name}
            disabled={!isNew}
            onChange={(e) => setName(e.target.value)}
            placeholder='temp-high'
            aria-invalid={nameError ? true : undefined}
            className={`h-8 font-mono text-[12px] ${nameError ? 'border-destructive' : ''}`}
          />
        </div>
        <Button size='sm' onClick={save} disabled={saving} className='h-8 gap-1.5'>
          <Save className='size-3.5' /> {isNew ? 'Create' : 'Save'}
        </Button>
        {!isNew && (
          <Button
            size='sm'
            variant='outline'
            className='text-sev-fault h-8 gap-1.5'
            onClick={() => setConfirmDelete(true)}
          >
            <Trash2 className='size-3.5' /> Delete
          </Button>
        )}
      </div>
      {nameError && (
        <p className='text-destructive text-[11px]'>{nameError}</p>
      )}

      {!isNew && referencing.length > 0 && (
        <ChangeImpact referencing={referencing} action='editing' />
      )}

      <ParamsEditor rows={paramRows} onChange={setParamRows} />

      <div className='flex flex-col gap-1'>
        <Label className='text-[11px]'>Script (Rhai)</Label>
        <CodeEditor
          value={script}
          onChange={setScript}
          language='rhai'
          paramNames={paramNames}
          ariaLabel='Rule script'
          minHeight={240}
        />
      </div>

      <ConfirmDialog
        open={confirmDelete}
        onOpenChange={setConfirmDelete}
        destructive
        title={`Delete rule "${rule?.name}"?`}
        desc={
          referencing.length > 0
            ? `${referencing.length} rule(s) compose this one and will break on the next tick. This cannot be undone.`
            : 'This permanently removes the rule. This cannot be undone.'
        }
        confirmText='Delete rule'
        isLoading={del.isPending}
        handleConfirm={() =>
          rule &&
          del.mutate(rule.name, {
            onSuccess: () => {
              toast.success(`Rule "${rule.name}" deleted`)
              setConfirmDelete(false)
              onDeleted()
            },
            onError: (e) =>
              toast.error(e instanceof ApiError ? e.message : 'Delete failed'),
          })
        }
      />
    </Card>
  )
}

/** A small key/required/description editor for the rule's param schema. */
function ParamsEditor({
  rows,
  onChange,
}: {
  rows: ParamRow[]
  onChange: (rows: ParamRow[]) => void
}) {
  const set = (i: number, patch: Partial<ParamRow>) =>
    onChange(rows.map((r, j) => (j === i ? { ...r, ...patch } : r)))
  const add = () => onChange([...rows, { key: '', required: false, description: '' }])
  const remove = (i: number) => onChange(rows.filter((_, j) => j !== i))

  return (
    <div className='flex flex-col gap-1.5'>
      <div className='flex items-center justify-between'>
        <Label className='text-[11px]'>Parameters</Label>
        <Button
          size='sm'
          variant='ghost'
          className='h-6 gap-1 px-2 text-[11px]'
          onClick={add}
        >
          <Plus className='size-3' /> Add
        </Button>
      </div>
      {rows.length === 0 ? (
        <p className='text-muted-foreground text-[11px]'>
          No parameters. The script reads them as <code>params.name</code>.
        </p>
      ) : (
        rows.map((r, i) => (
          <div key={i} className='flex items-center gap-2'>
            <Input
              value={r.key}
              onChange={(e) => set(i, { key: e.target.value })}
              placeholder='param'
              className='h-7 w-32 font-mono text-[11.5px]'
            />
            <Input
              value={r.description}
              onChange={(e) => set(i, { description: e.target.value })}
              placeholder='description'
              className='h-7 flex-1 text-[11.5px]'
            />
            <label className='flex items-center gap-1.5 text-[10.5px] text-muted-foreground'>
              <Switch
                checked={r.required}
                onCheckedChange={(v) => set(i, { required: v })}
                aria-label={`${r.key || 'param'} required`}
              />
              req
            </label>
            <Button
              size='icon'
              variant='ghost'
              className='size-7'
              aria-label='Remove parameter'
              onClick={() => remove(i)}
            >
              <X className='size-3.5' />
            </Button>
          </div>
        ))
      )}
    </div>
  )
}