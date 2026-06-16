// Admin · Rules Studio — author Rhai rules over time-window bindings, dry-run a
// draft against real history (verdict + the frame it saw), and explore the rows a
// rule reads with a SQL workbench. Rules persist as `kind:"rule"` records and are
// gated on rule-define; the studio talks to the dedicated `/rules` surface
// (crates/rubix-server/src/http/rules/*), never raw records.
//
// Layout mirrors the Sparks master/detail language the old feature used, adapted
// to this app's conventions: a searchable rule list on the left, the editor +
// debugger on the right, and a tab to the query workbench. The live editor draft
// lifts to the page so the debugger dry-runs exactly what is on screen.

import { getRouteApi } from '@tanstack/react-router'
import { useMemo, useState } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import {
  Database,
  FileCode2,
  GitFork,
  Play,
  Plus,
  Save,
  ScrollText,
  Search,
  Trash2,
  TriangleAlert,
  X,
} from 'lucide-react'
import {
  Area,
  AreaChart,
  CartesianGrid,
  ReferenceDot,
  ResponsiveContainer,
  XAxis,
  YAxis,
} from 'recharts'
import { useApi } from '../../../api/ConnectionContext'
import { ApiError } from '../../../api/client'
import {
  AGGREGATES,
  GRAINS,
  TABLES,
  createRule,
  deleteRule,
  dryRunRule,
  listRules,
  referencingRules,
  ruleCatalog,
  updateRule,
  type Aggregate,
  type Binding,
  type CanonicalTable,
  type DryRunResponse,
  type Grain,
  type Rule,
  type RuleCatalog,
} from '../../../api/rules'
import { runQuery, type QueryResponse } from '../../../api/query'
import { usePageHeader } from '../../../components/shell/page-header'
import { ErrorView } from '../../../components/ui/StateView'
import { Badge } from '../../../components/ui/badge'
import { Button } from '../../../components/ui/button'
import { Combobox } from '../../../components/ui/combobox'
import { Input } from '../../../components/ui/input'
import { Label } from '../../../components/ui/label'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '../../../components/ui/select'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '../../../components/ui/tabs'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '../../../components/ui/table'
import { SqlEditor } from '../../../components/sql/SqlEditor'
import { cn } from '../../../lib/cn'

const route = getRouteApi('/t/$tenant/admin/rules')

const NEW_SCRIPT = `// A rule reads each binding by name and returns a decision.
// Return a bool, or a map { fired, value, reason }.
#{ fired: temp > 23.5, value: temp, reason: "zone above 23.5°C" }
`

// Default to a reading-table binding: the typed time-series `value`, rolled up
// over the window and scoped to one series (a point id) via the filter. `week`
// rolls the whole trailing window into one bucket. Edit the series to a real
// point id (see the seeded rules for examples, e.g. acme--hq--ahu-1--zone-temp).
const NEW_BINDING = (): Binding => ({
  name: 'temp',
  table: 'readings',
  field: 'value',
  grain: 'week',
  aggregate: 'max',
  filter_field: 'series',
  filter_value: 'acme--hq--ahu-1--zone-temp',
})

// The draft the editor edits and the debugger dry-runs. `null` rule + creating =
// the "new rule" draft.
interface Draft {
  name: string
  script: string
  inputs: Binding[]
  subrules: string[]
  output: string
}

function draftFrom(rule: Rule | null): Draft {
  return rule
    ? {
        name: rule.name,
        script: rule.script,
        inputs: rule.inputs,
        subrules: rule.subrules,
        output: rule.output,
      }
    : {
        name: '',
        script: NEW_SCRIPT,
        inputs: [NEW_BINDING()],
        subrules: [],
        output: 'insight',
      }
}

export function AdminRules() {
  const { tenant } = route.useParams()
  const api = useApi(tenant)
  const qc = useQueryClient()

  const [filter, setFilter] = useState('')
  const [selected, setSelected] = useState<string | null>(null)
  const [creating, setCreating] = useState(false)

  const rules = useQuery({ queryKey: ['rules', tenant], queryFn: () => listRules(api) })

  const filtered = useMemo(
    () =>
      (rules.data ?? [])
        .filter((r) => r.name.toLowerCase().includes(filter.toLowerCase()))
        .sort((a, b) => a.name.localeCompare(b.name)),
    [rules.data, filter],
  )

  // The effective rule: the explicit selection, else the first rule once loaded,
  // unless mid-create.
  const effective: Rule | null = creating
    ? null
    : ((selected !== null && filtered.find((r) => r.name === selected)) ||
        (filtered.length ? filtered[0]! : null) ||
        null)

  function startNew() {
    setCreating(true)
    setSelected(null)
  }
  function onSelect(name: string) {
    setCreating(false)
    setSelected(name)
  }

  usePageHeader({ crumbs: ['Admin', 'Rules'] })

  return (
    <div className="px-6 py-6">
      <div className="mx-auto max-w-[1400px]">
        <div className="mb-5 flex items-center gap-3">
          <div className="grid size-11 place-items-center rounded-xl border border-border bg-card">
            <GitFork size={20} className="text-muted-foreground" />
          </div>
          <div>
            <h1 className="text-[22px] font-semibold tracking-tight">Rules Studio</h1>
            <div className="text-[13px] text-muted-foreground">
              Author, debug, and explore rules over time-window bindings.
            </div>
          </div>
        </div>

        <Tabs defaultValue="rules">
          <TabsList>
            <TabsTrigger value="rules" className="gap-1.5">
              <ScrollText size={14} /> Rules
            </TabsTrigger>
            <TabsTrigger value="query" className="gap-1.5">
              <Database size={14} /> Query workbench
            </TabsTrigger>
          </TabsList>

          <TabsContent value="rules">
            <div className="grid gap-4 lg:grid-cols-[300px_1fr]">
              {/* master list */}
              <div className="flex flex-col gap-2.5">
                <div className="flex items-center gap-2">
                  <div className="relative flex-1">
                    <Search className="absolute left-2 top-1/2 size-3.5 -translate-y-1/2 text-muted-foreground" />
                    <Input
                      value={filter}
                      onChange={(e) => setFilter(e.target.value)}
                      placeholder="Search rules"
                      aria-label="Search rules"
                      className="h-9 ps-7 text-[13px]"
                    />
                  </div>
                  <Button size="sm" onClick={startNew} className="h-9 gap-1">
                    <Plus className="size-3.5" /> New
                  </Button>
                </div>
                <div className="max-h-[70vh] space-y-1 overflow-y-auto rounded-xl border border-border bg-card/40 p-1.5">
                  {rules.isLoading ? (
                    <ListSkeleton />
                  ) : (
                    <RuleList
                      rules={filtered}
                      selectedName={creating ? null : (effective?.name ?? null)}
                      onSelect={onSelect}
                    />
                  )}
                </div>
              </div>

              {/* detail */}
              <div className="space-y-4">
                {creating || effective ? (
                  <RuleDetail
                    // Remount per rule (and for the new draft) so the editor seeds
                    // from the freshly-selected rule via initial state.
                    key={creating ? '__new__' : effective!.name}
                    api={api}
                    tenant={tenant}
                    rule={creating ? null : effective}
                    onSaved={(name) => {
                      setCreating(false)
                      setSelected(name)
                      void qc.invalidateQueries({ queryKey: ['rules', tenant] })
                    }}
                    onDeleted={() => {
                      setCreating(false)
                      setSelected(null)
                      void qc.invalidateQueries({ queryKey: ['rules', tenant] })
                    }}
                  />
                ) : (
                  <div className="grid place-items-center rounded-xl border border-border bg-card/40 p-12 text-center">
                    <div>
                      <ScrollText className="mx-auto size-8 text-muted-foreground" />
                      <p className="mt-3 text-sm text-muted-foreground">
                        Select a rule or create a new one.
                      </p>
                      <Button size="sm" className="mt-3 gap-1" onClick={startNew}>
                        <Plus className="size-3.5" /> New rule
                      </Button>
                    </div>
                  </div>
                )}
              </div>
            </div>
          </TabsContent>

          <TabsContent value="query">
            <SqlWorkbench api={api} />
          </TabsContent>
        </Tabs>
      </div>
    </div>
  )
}

// ── master list ──────────────────────────────────────────────────────────────

// Cosmetic severity accent from the highest-severity finding the script names —
// purely a left-border cue; the authoritative verdict is the dry-run, not source.
function emittedAccent(script: string): string {
  if (/fault/i.test(script)) return 'border-l-destructive'
  if (/warn/i.test(script)) return 'border-l-amber-500'
  return 'border-l-border'
}

function RuleList({
  rules,
  selectedName,
  onSelect,
}: {
  rules: Rule[]
  selectedName: string | null
  onSelect: (name: string) => void
}) {
  if (rules.length === 0) {
    return <p className="py-12 text-center text-sm text-muted-foreground">No rules yet.</p>
  }
  return (
    <>
      {rules.map((rule) => {
        const active = rule.name === selectedName
        return (
          <button
            key={rule.id}
            type="button"
            onClick={() => onSelect(rule.name)}
            aria-current={active}
            className={cn(
              'flex w-full items-start gap-2.5 rounded-md border border-l-2 border-transparent px-2.5 py-2 text-left',
              'hover:bg-muted/50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring',
              emittedAccent(rule.script),
              active && 'bg-muted',
            )}
          >
            <FileCode2 className="mt-0.5 size-4 shrink-0 text-muted-foreground" />
            <div className="min-w-0">
              <div className="mono truncate text-[12.5px] font-medium">{rule.name}</div>
              <div className="mt-0.5 text-[10.5px] text-muted-foreground">
                {rule.inputs.length} input{rule.inputs.length === 1 ? '' : 's'} · {rule.output}
              </div>
            </div>
          </button>
        )
      })}
    </>
  )
}

function ListSkeleton() {
  return (
    <div className="space-y-1.5 p-1">
      {Array.from({ length: 6 }).map((_, i) => (
        <div key={i} className="h-11 animate-pulse rounded-md bg-muted/50" />
      ))}
    </div>
  )
}

// ── detail: editor + debugger ────────────────────────────────────────────────

function RuleDetail({
  api,
  tenant,
  rule,
  onSaved,
  onDeleted,
}: {
  api: ReturnType<typeof useApi>
  tenant: string
  rule: Rule | null
  onSaved: (name: string) => void
  onDeleted: () => void
}) {
  const isNew = rule === null
  const [draft, setDraft] = useState<Draft>(() => draftFrom(rule))
  const [nameError, setNameError] = useState<string | undefined>()

  const referencing = useQuery({
    queryKey: ['rule-referencing', tenant, rule?.name],
    queryFn: () => referencingRules(api, rule!.name),
    enabled: !isNew && !!rule,
  })

  const save = useMutation<Rule, Error, void>({
    mutationFn: () => {
      if (isNew) {
        return createRule(api, {
          name: draft.name.trim(),
          script: draft.script,
          inputs: draft.inputs,
          subrules: draft.subrules,
          output: draft.output.trim() || 'insight',
        })
      }
      return updateRule(api, rule!.name, {
        script: draft.script,
        inputs: draft.inputs,
        subrules: draft.subrules,
        output: draft.output.trim() || 'insight',
      })
    },
    onSuccess: (saved) => {
      setNameError(undefined)
      onSaved(saved.name)
    },
    onError: (e) => {
      if (e instanceof ApiError && e.status === 409) {
        setNameError('A rule with this name already exists in this tenant.')
      }
    },
  })

  const remove = useMutation<void, Error, void>({
    mutationFn: () => deleteRule(api, rule!.name),
    onSuccess: onDeleted,
  })

  function onSaveClick() {
    setNameError(undefined)
    if (isNew && !/^[a-z0-9-]+$/.test(draft.name.trim())) {
      setNameError('Use a lowercase slug (a–z, 0–9, hyphen).')
      return
    }
    save.mutate()
  }

  const refs = referencing.data ?? []

  return (
    <>
      <div className="space-y-3 rounded-xl border border-border bg-card/40 p-4">
        <div className="flex flex-wrap items-end gap-2">
          <div className="flex min-w-[200px] flex-1 flex-col gap-1">
            <Label htmlFor="rule-name" className="text-[11px]">
              Name
            </Label>
            <Input
              id="rule-name"
              value={draft.name}
              disabled={!isNew}
              onChange={(e) => setDraft({ ...draft, name: e.target.value })}
              placeholder="temp-high"
              aria-invalid={nameError ? true : undefined}
              className={cn('mono h-9 text-[13px]', nameError && 'border-destructive')}
            />
          </div>
          <div className="flex w-44 flex-col gap-1">
            <Label htmlFor="rule-output" className="text-[11px]">
              Output (insight kind)
            </Label>
            <Input
              id="rule-output"
              value={draft.output}
              onChange={(e) => setDraft({ ...draft, output: e.target.value })}
              placeholder="high-temperature"
              className="mono h-9 text-[13px]"
            />
          </div>
          <Button onClick={onSaveClick} disabled={save.isPending} className="h-9 gap-1.5">
            <Save className="size-3.5" /> {isNew ? 'Create' : 'Save'}
          </Button>
          {!isNew && (
            <Button
              variant="outline"
              onClick={() => remove.mutate()}
              disabled={remove.isPending}
              className="h-9 gap-1.5 text-destructive"
            >
              <Trash2 className="size-3.5" /> Delete
            </Button>
          )}
        </div>
        {nameError && <p className="text-[11px] text-destructive">{nameError}</p>}
        {save.error && !nameError && (
          <p className="mono text-[11px] text-destructive">
            {save.error instanceof ApiError ? save.error.message : 'Save failed'}
          </p>
        )}

        {refs.length > 0 && <ChangeImpact referencing={refs} />}

        <BindingsEditor
          api={api}
          tenant={tenant}
          inputs={draft.inputs}
          onChange={(inputs) => setDraft({ ...draft, inputs })}
        />

        <div className="flex flex-col gap-1">
          <Label className="text-[11px]">Script (Rhai)</Label>
          <div className="rounded-md border border-border">
            <SqlEditor
              value={draft.script}
              onChange={(script) => setDraft({ ...draft, script })}
              minHeight="200px"
            />
          </div>
        </div>
      </div>

      <RuleDebugger api={api} draft={draft} />
    </>
  )
}

// The bindings editor: each row declares a script input from a table/field/grain/
// aggregate, the rubix-rules Binding shape the dry-run resolves.
function BindingsEditor({
  api,
  tenant,
  inputs,
  onChange,
}: {
  api: ReturnType<typeof useApi>
  tenant: string
  inputs: Binding[]
  onChange: (inputs: Binding[]) => void
}) {
  const set = (i: number, patch: Partial<Binding>) =>
    onChange(inputs.map((b, j) => (j === i ? { ...b, ...patch } : b)))
  const add = () => onChange([...inputs, NEW_BINDING()])
  const remove = (i: number) => onChange(inputs.filter((_, j) => j !== i))

  // Discover what each table in play actually holds, so the field/filter inputs
  // suggest real fields and series instead of being typed blind. One query over
  // the distinct tables used, refetched only when that set changes; each input is
  // still free text (a datalist suggests, never constrains), so an unseeded
  // series stays typeable.
  const tables = useMemo(() => [...new Set(inputs.map((b) => b.table))].sort(), [inputs])
  const catalogs = useQuery({
    queryKey: ['rule-catalog', tenant, tables],
    enabled: tables.length > 0,
    staleTime: 30_000,
    queryFn: async () => {
      const entries = await Promise.all(
        tables.map(async (t) => [t, await ruleCatalog(api, t)] as const),
      )
      return Object.fromEntries(entries) as Record<string, RuleCatalog>
    },
  })
  const byTable = catalogs.data ?? {}

  return (
    <div className="flex flex-col gap-1.5">
      <div className="flex items-center justify-between">
        <Label className="text-[11px]">Inputs (window bindings)</Label>
        <Button variant="ghost" size="sm" className="h-7 gap-1 px-2 text-[11px]" onClick={add}>
          <Plus className="size-3" /> Add
        </Button>
      </div>
      {inputs.length === 0 ? (
        <p className="text-[11px] text-muted-foreground">
          No inputs. The script reads each binding by its name.
        </p>
      ) : (
        inputs.map((b, i) => {
          const catalog = byTable[b.table]
          const loading = catalogs.isFetching && !catalog
          const fieldOptions = catalog?.fields ?? []
          const filterKeys = catalog?.filters.map((f) => f.key) ?? []
          const activeFilter = catalog?.filters.find((f) => f.key === b.filter_field)
          const valueOptions = activeFilter?.values ?? []
          return (
            <div key={i} className="flex flex-wrap items-center gap-1.5">
              <Input
                value={b.name}
                onChange={(e) => set(i, { name: e.target.value })}
                placeholder="name"
                className="mono h-8 w-28 text-[12px]"
                aria-label="binding name"
              />
              <EnumSelect
                value={b.table}
                options={TABLES}
                onChange={(v) => set(i, { table: v as CanonicalTable })}
                width="w-[124px]"
              />
              <Combobox
                value={b.field}
                onChange={(v) => set(i, { field: v })}
                options={fieldOptions}
                placeholder="content.field"
                emptyLabel={loading ? 'loading…' : undefined}
                aria-label="binding field"
                className="w-32"
              />
              <EnumSelect
                value={b.grain}
                options={GRAINS}
                onChange={(v) => set(i, { grain: v as Grain })}
                width="w-[96px]"
              />
              <EnumSelect
                value={b.aggregate}
                options={AGGREGATES}
                onChange={(v) => set(i, { aggregate: v as Aggregate })}
                width="w-[96px]"
              />
              {/* Optional filter: scope a reading binding to one series
                  (where series = <point id>), or narrow a record series to one
                  category (where measure = temp). Empty = the whole table. The
                  comboboxes offer the keys and values discovered in the data. */}
              <span className="text-[10.5px] text-muted-foreground">where</span>
              <Combobox
                value={b.filter_field ?? ''}
                onChange={(v) => set(i, { filter_field: v })}
                options={filterKeys}
                placeholder={b.table === 'readings' ? 'series' : 'measure'}
                emptyLabel={loading ? 'loading…' : undefined}
                aria-label="filter field"
                className="w-24"
              />
              <span className="text-[10.5px] text-muted-foreground">=</span>
              <Combobox
                value={b.filter_value ?? ''}
                onChange={(v) => set(i, { filter_value: v })}
                options={valueOptions}
                placeholder={b.table === 'readings' ? 'site--equip--point' : 'temp'}
                emptyLabel={loading ? 'loading…' : undefined}
                truncatedNote={
                  activeFilter?.truncated
                    ? `First ${valueOptions.length} values shown — type to use an unlisted one.`
                    : undefined
                }
                aria-label="filter value"
                className="w-44"
              />
              <Button
                variant="ghost"
                size="icon"
                className="size-8"
                aria-label="remove binding"
                onClick={() => remove(i)}
              >
                <X className="size-3.5" />
              </Button>
            </div>
          )
        })
      )}
      {catalogs.isError && (
        <p className="text-[10.5px] text-muted-foreground">
          Couldn't load field/series suggestions — you can still type bindings by hand.
        </p>
      )}
    </div>
  )
}

function EnumSelect({
  value,
  options,
  onChange,
  width,
}: {
  value: string
  options: readonly string[]
  onChange: (v: string) => void
  width: string
}) {
  return (
    <Select value={value} onValueChange={onChange}>
      <SelectTrigger className={cn('mono h-8 text-[12px]', width)}>
        <SelectValue />
      </SelectTrigger>
      <SelectContent>
        {options.map((o) => (
          <SelectItem key={o} value={o} className="mono text-[12px]">
            {o}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  )
}

// ── change impact ────────────────────────────────────────────────────────────

function ChangeImpact({ referencing }: { referencing: Rule[] }) {
  return (
    <div className="rounded-md border border-amber-500/40 bg-amber-500/10 p-3">
      <div className="flex items-center gap-2">
        <GitFork className="size-4 text-amber-600" />
        <p className="text-[12px] font-medium">
          {referencing.length} rule{referencing.length === 1 ? '' : 's'} compose this one —
          editing it changes them on the next tick.
        </p>
      </div>
      <div className="mt-2 flex flex-wrap gap-1.5">
        {referencing.map((r) => (
          <span
            key={r.id}
            className="mono rounded border border-border bg-card px-1.5 py-0.5 text-[10.5px] text-muted-foreground"
          >
            {r.name}
          </span>
        ))}
      </div>
    </div>
  )
}

// ── debugger ─────────────────────────────────────────────────────────────────

function RuleDebugger({ api, draft }: { api: ReturnType<typeof useApi>; draft: Draft }) {
  const dryRun = useMutation<DryRunResponse, Error, void>({
    mutationFn: () =>
      dryRunRule(api, draft.name.trim() || 'draft', {
        script: draft.script,
        inputs: draft.inputs,
        subrules: draft.subrules,
      }),
  })

  return (
    <div className="space-y-3 rounded-xl border border-border bg-card/40 p-4">
      <div className="flex items-center justify-between">
        <span className="text-[10px] font-semibold uppercase tracking-wide text-muted-foreground">
          Debugger · dry-run
        </span>
        {dryRun.data && <Verdict data={dryRun.data} />}
      </div>

      <div className="flex items-center gap-2">
        <Button size="sm" onClick={() => dryRun.mutate()} disabled={dryRun.isPending} className="gap-1.5">
          <Play className="size-3.5" /> {dryRun.isPending ? 'Running…' : 'Run'}
        </Button>
        <span className="text-[11px] text-muted-foreground">
          Resolves the bindings against your visible history — records no insight.
        </span>
      </div>

      {dryRun.isError && <DryRunError error={dryRun.error} />}
      {dryRun.data && <FrameView data={dryRun.data} />}
      {!dryRun.data && !dryRun.isError && (
        <p className="py-6 text-center text-[12px] text-muted-foreground">
          Run to see the verdict and the window frame the rule decided on.
        </p>
      )}
    </div>
  )
}

function Verdict({ data }: { data: DryRunResponse }) {
  if (!data.fired) {
    return (
      <Badge variant="muted" className="h-5 gap-1 px-2 text-[10.5px]">
        Clear — did not fire
      </Badge>
    )
  }
  return (
    <div className="flex items-center gap-2">
      <Badge variant="destructive" className="h-5 px-2 text-[10.5px]">
        Fired
      </Badge>
      <span className="text-[12.5px] font-medium">{data.reason || 'fired'}</span>
      <span className="mono text-[11px] text-muted-foreground">= {data.value}</span>
    </div>
  )
}

function DryRunError({ error }: { error: unknown }) {
  const message = error instanceof ApiError ? error.message : 'Dry-run failed.'
  const category = /compile/i.test(message)
    ? 'compile'
    : /window|no window bucket|binding/i.test(message)
      ? 'binding'
      : 'runtime'
  return (
    <div
      role="alert"
      className="flex items-start gap-2 rounded-md border border-destructive/40 bg-destructive/10 p-2.5"
    >
      <TriangleAlert className="mt-0.5 size-4 shrink-0 text-destructive" />
      <div className="min-w-0">
        <Badge variant="destructive" className="h-4 px-1.5 text-[9.5px] capitalize">
          {category}
        </Badge>
        <p className="mono mt-1 break-words text-[11px] text-destructive">{message}</p>
      </div>
    </div>
  )
}

// Chart the resolved frame of the first binding — the window buckets the rule saw,
// with the value the decision turned on marked when it appears in the series.
function FrameView({ data }: { data: DryRunResponse }) {
  const input = data.inputs[0]
  const points = useMemo(
    () =>
      (input?.buckets ?? []).map((b, i) => ({
        i,
        t: new Date(b.bucket_start / 1000).toLocaleTimeString([], {
          hour: '2-digit',
          minute: '2-digit',
        }),
        value: b.avg,
      })),
    [input],
  )

  const firedIndex =
    data.fired && input ? points.findIndex((p) => p.value === data.value) : -1

  if (!input) {
    return (
      <p className="rounded-md border border-border p-3 text-center text-[12px] text-muted-foreground">
        No bindings to chart.
      </p>
    )
  }
  if (points.length < 2) {
    return (
      <div className="rounded-md border border-border p-3 text-center text-[12px] text-muted-foreground">
        {points.length === 0
          ? 'Empty frame — no readings in the window.'
          : `${points.length} bucket; not enough points to chart.`}
      </div>
    )
  }

  return (
    <div className="space-y-2">
      <ResponsiveContainer width="100%" height={200}>
        <AreaChart data={points} margin={{ top: 6, right: 8, left: -14, bottom: 0 }}>
          <defs>
            <linearGradient id="frameFill" x1="0" y1="0" x2="0" y2="1">
              <stop offset="0%" stopColor="var(--chart-1)" stopOpacity={0.28} />
              <stop offset="100%" stopColor="var(--chart-1)" stopOpacity={0} />
            </linearGradient>
          </defs>
          <CartesianGrid stroke="var(--border)" vertical={false} />
          <XAxis
            dataKey="t"
            tickLine={false}
            axisLine={false}
            fontSize={10}
            minTickGap={44}
            tick={{ fill: 'var(--muted-foreground)' }}
          />
          <YAxis
            tickLine={false}
            axisLine={false}
            fontSize={10}
            width={48}
            tick={{ fill: 'var(--muted-foreground)' }}
            domain={['auto', 'auto']}
          />
          <Area
            type="monotone"
            dataKey="value"
            stroke="var(--chart-1)"
            strokeWidth={1.8}
            fill="url(#frameFill)"
            isAnimationActive={false}
          />
          {firedIndex >= 0 && (
            <ReferenceDot
              x={points[firedIndex]!.t}
              y={points[firedIndex]!.value}
              r={4}
              fill="var(--destructive)"
              stroke="var(--background)"
            />
          )}
        </AreaChart>
      </ResponsiveContainer>
      <p className="text-[10.5px] text-muted-foreground">
        binding <span className="mono">{input.name}</span> · {input.buckets.length} bucket(s) · avg
        series
      </p>
    </div>
  )
}

// ── SQL workbench tab ────────────────────────────────────────────────────────

// Readings live in the typed `reading` table with top-level `series`/`value`/`at`
// columns — explore them here to find a series id for a rule binding.
const DEFAULT_SQL = 'SELECT series, value, at FROM reading ORDER BY at DESC LIMIT 20'

// A lightweight query console so an author can explore the rows a binding reads
// before writing the rule. Reuses the shared SqlEditor and the POST /query path.
function SqlWorkbench({ api }: { api: ReturnType<typeof useApi> }) {
  const [sql, setSql] = useState(DEFAULT_SQL)
  const run = useMutation<QueryResponse, Error, void>({
    mutationFn: () => runQuery(api, sql),
  })
  const rows = run.data?.rows ?? []
  const columns = useMemo(() => {
    const set = new Set<string>()
    for (const row of rows) for (const k of Object.keys(row)) set.add(k)
    return [...set]
  }, [rows])

  return (
    <div className="space-y-3">
      <div className="rounded-xl border border-border">
        <SqlEditor value={sql} onChange={setSql} onRun={() => run.mutate()} />
      </div>
      <div className="flex items-center gap-3">
        <Button onClick={() => run.mutate()} disabled={run.isPending} className="gap-1.5">
          <Play size={15} /> {run.isPending ? 'Running…' : 'Run query'}
        </Button>
        <span className="text-xs text-muted-foreground">⌘/Ctrl + Enter</span>
        {run.data && (
          <span className="ml-auto text-xs text-muted-foreground">
            {rows.length} {rows.length === 1 ? 'row' : 'rows'}
          </span>
        )}
      </div>

      {run.error && <ErrorView error={run.error} />}

      {run.data && (
        <div className="rounded-xl border border-border bg-card/40">
          {rows.length === 0 ? (
            <p className="py-12 text-center text-sm text-muted-foreground">No rows.</p>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  {columns.map((c) => (
                    <TableHead key={c} className="mono">
                      {c}
                    </TableHead>
                  ))}
                </TableRow>
              </TableHeader>
              <TableBody>
                {rows.map((row, i) => (
                  <TableRow key={i}>
                    {columns.map((c) => (
                      <TableCell key={c} className="mono text-xs">
                        {renderCell(row[c])}
                      </TableCell>
                    ))}
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </div>
      )}
    </div>
  )
}

function renderCell(value: unknown): string {
  if (value === null || value === undefined) return '—'
  if (typeof value === 'object') return JSON.stringify(value)
  return String(value)
}
