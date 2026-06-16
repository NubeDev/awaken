// Transform pipeline editor (§1) — author the portable transform spec stored on a
// chart. An ordered list of transforms; aggregate ops (filter/groupBy/reduce) run
// server-side, cosmetic ops (rename/calculated/organize) client-side — the editor
// labels which tier each runs in so the split is visible. Kept compact: a kind
// picker per row plus that kind's fields. The spec is the durable contract; this
// is just its authoring surface.

import { Plus, Trash2 } from 'lucide-react'

import type { Agg, CalcOp, CompareOp, ReduceCalc, Transform } from './transforms'
import { isAggregate } from './transforms'
import { Button } from '../ui/button'
import { Input } from '../ui/input'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '../ui/select'

const KINDS: ReadonlyArray<{ value: Transform['kind']; label: string }> = [
  { value: 'filter', label: 'Filter' },
  { value: 'groupBy', label: 'Group by' },
  { value: 'reduce', label: 'Reduce' },
  { value: 'rename', label: 'Rename' },
  { value: 'calculated', label: 'Calculated' },
  { value: 'organize', label: 'Organize' },
]

const COMPARE_OPS: CompareOp[] = ['=', '!=', '>', '>=', '<', '<=']
const CALC_OPS: CalcOp[] = ['+', '-', '*', '/']
const AGGS: Agg[] = ['sum', 'avg', 'min', 'max', 'count']
const REDUCE_CALCS: ReduceCalc[] = ['first', 'last', 'sum', 'avg', 'min', 'max', 'count']

// A fresh transform of the given kind, with empty fields.
function blank(kind: Transform['kind']): Transform {
  switch (kind) {
    case 'filter':
      return { kind, field: '', op: '=', value: '' }
    case 'groupBy':
      return { kind, by: '', field: '', agg: 'sum', as: '' }
    case 'reduce':
      return { kind, field: '', calc: 'sum', as: '' }
    case 'rename':
      return { kind, from: '', to: '' }
    case 'calculated':
      return { kind, field: '', left: '', op: '+', right: '' }
    case 'organize':
      return { kind, order: [] }
  }
}

interface TransformEditorProps {
  value: ReadonlyArray<Transform> | undefined
  onChange: (transforms: Transform[]) => void
}

export function TransformEditor({ value, onChange }: TransformEditorProps) {
  const list = value ?? []

  const update = (i: number, t: Transform) => onChange(list.map((x, j) => (j === i ? t : x)))
  const remove = (i: number) => onChange(list.filter((_, j) => j !== i))
  const add = () => onChange([...list, blank('filter')])

  return (
    <div className="flex flex-col gap-2">
      <div className="flex items-center justify-between">
        <span className="text-xs font-medium text-muted-foreground">Transforms</span>
        <Button variant="ghost" onClick={add} className="h-7 gap-1 text-xs">
          <Plus size={13} /> Add transform
        </Button>
      </div>
      {list.length === 0 ? (
        <p className="text-xs text-muted-foreground">
          No transforms. Aggregate ops run server-side; cosmetic ops in the browser.
        </p>
      ) : (
        list.map((t, i) => (
          <div key={i} className="flex flex-wrap items-center gap-1.5 rounded-lg border border-border p-1.5">
            <Select value={t.kind} onValueChange={(k) => update(i, blank(k as Transform['kind']))}>
              <SelectTrigger className="h-8 w-[120px]">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {KINDS.map((k) => (
                  <SelectItem key={k.value} value={k.value}>
                    {k.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
            <TransformFields transform={t} onChange={(next) => update(i, next)} />
            <span className="ml-auto text-[10px] uppercase tracking-wide text-muted-foreground">
              {isAggregate(t) ? 'server' : 'client'}
            </span>
            <Button variant="ghost" onClick={() => remove(i)} className="h-7 w-7 p-0 text-muted-foreground">
              <Trash2 size={13} />
            </Button>
          </div>
        ))
      )}
    </div>
  )
}

// The per-kind field row. Each branch edits one transform variant in place.
function TransformFields({
  transform,
  onChange,
}: {
  transform: Transform
  onChange: (t: Transform) => void
}) {
  const field = (placeholder: string, val: string, set: (v: string) => void) => (
    <Input
      value={val}
      placeholder={placeholder}
      onChange={(e) => set(e.target.value)}
      className="h-8 w-[110px]"
    />
  )

  switch (transform.kind) {
    case 'filter':
      return (
        <>
          {field('field', transform.field, (field) => onChange({ ...transform, field }))}
          <OpSelect ops={COMPARE_OPS} value={transform.op} onChange={(op) => onChange({ ...transform, op })} />
          {field('value', transform.value, (value) => onChange({ ...transform, value }))}
        </>
      )
    case 'groupBy':
      return (
        <>
          {field('by', transform.by, (by) => onChange({ ...transform, by }))}
          <OpSelect ops={AGGS} value={transform.agg} onChange={(agg) => onChange({ ...transform, agg })} />
          {field('field', transform.field, (field) => onChange({ ...transform, field }))}
          {field('as', transform.as, (as) => onChange({ ...transform, as }))}
        </>
      )
    case 'reduce':
      return (
        <>
          <OpSelect
            ops={REDUCE_CALCS}
            value={transform.calc}
            onChange={(calc) => onChange({ ...transform, calc })}
          />
          {field('field', transform.field, (field) => onChange({ ...transform, field }))}
          {field('as', transform.as, (as) => onChange({ ...transform, as }))}
        </>
      )
    case 'rename':
      return (
        <>
          {field('from', transform.from, (from) => onChange({ ...transform, from }))}
          {field('to', transform.to, (to) => onChange({ ...transform, to }))}
        </>
      )
    case 'calculated':
      return (
        <>
          {field('field', transform.field, (field) => onChange({ ...transform, field }))}
          {field('left', transform.left, (left) => onChange({ ...transform, left }))}
          <OpSelect ops={CALC_OPS} value={transform.op} onChange={(op) => onChange({ ...transform, op })} />
          {field('right', transform.right, (right) => onChange({ ...transform, right }))}
        </>
      )
    case 'organize':
      return field('columns (comma)', transform.order.join(','), (v) =>
        onChange({ ...transform, order: v.split(',').map((s) => s.trim()).filter(Boolean) }),
      )
  }
}

// A narrow select over a fixed set of literal options (op/agg/calc).
function OpSelect<T extends string>({
  ops,
  value,
  onChange,
}: {
  ops: ReadonlyArray<T>
  value: T
  onChange: (v: T) => void
}) {
  return (
    <Select value={value} onValueChange={(v) => onChange(v as T)}>
      <SelectTrigger className="h-8 w-[90px]">
        <SelectValue />
      </SelectTrigger>
      <SelectContent>
        {ops.map((o) => (
          <SelectItem key={o} value={o}>
            {o}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  )
}
