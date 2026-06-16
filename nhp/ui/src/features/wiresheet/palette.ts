/**
 * The wiresheet node catalogue — a Sedona/Niagara-style "kit" of function blocks
 * the user drags onto the canvas to compose extra EMS logic (custom alarming,
 * custom reporting) on top of the live portfolio. This is a FRONTEND-ONLY demo
 * surface: the blocks describe a plausible programmable-logic palette and render
 * richly, but nothing here executes — the canvas is illustrative.
 *
 * A block is grouped into a kit category, has a tinted accent (so the canvas
 * reads at a glance), a lucide icon, and a typed set of input/output ports. The
 * editor (wiresheet-canvas) instantiates a node from a spec and wires ports with
 * React-Flow edges. Source blocks (live meter registers) are produced separately
 * from the real portfolio (source-nodes.ts); everything else is from this kit.
 */
import {
  Activity,
  AlarmClock,
  ArrowRightLeft,
  Bell,
  BellRing,
  Binary,
  CalendarClock,
  Database,
  Divide,
  FileBarChart,
  Filter,
  FunctionSquare,
  Gauge,
  GitCompareArrows,
  Hash,
  Mail,
  Minus,
  Plus,
  Send,
  Sigma,
  SlidersHorizontal,
  Split,
  TimerReset,
  TrendingUp,
  Webhook,
  Workflow,
  X,
  Zap,
  type LucideIcon,
} from 'lucide-react'

export type PortKind = 'number' | 'bool' | 'event' | 'any'

export interface PortSpec {
  id: string
  label: string
  kind: PortKind
}

export type KitCategory =
  | 'Sources'
  | 'Math'
  | 'Logic'
  | 'Analytics'
  | 'Alarming'
  | 'Reporting'

export interface BlockSpec {
  type: string
  name: string
  category: KitCategory
  icon: LucideIcon
  /** A short one-liner shown in the palette + node footer. */
  blurb: string
  /** chart-N token (1..5) used for the node accent + port colour. */
  accent: 1 | 2 | 3 | 4 | 5
  inputs: PortSpec[]
  outputs: PortSpec[]
  /** Optional tunable shown in the node body (demo only — not evaluated). */
  config?: { label: string; value: string; suffix?: string }
}

const inN = (n: number, kind: PortKind = 'number'): PortSpec[] =>
  Array.from({ length: n }, (_, i) => ({
    id: `in${i + 1}`,
    label: String.fromCharCode(65 + i),
    kind,
  }))

const out = (id: string, label: string, kind: PortKind = 'number'): PortSpec => ({
  id,
  label,
  kind,
})

export const BLOCKS: BlockSpec[] = [
  // ---- Math -------------------------------------------------------------
  {
    type: 'add',
    name: 'Add',
    category: 'Math',
    icon: Plus,
    blurb: 'Sum of inputs',
    accent: 2,
    inputs: inN(2),
    outputs: [out('out', 'Σ')],
  },
  {
    type: 'subtract',
    name: 'Subtract',
    category: 'Math',
    icon: Minus,
    blurb: 'A − B',
    accent: 2,
    inputs: inN(2),
    outputs: [out('out', 'Δ')],
  },
  {
    type: 'multiply',
    name: 'Multiply',
    category: 'Math',
    icon: X,
    blurb: 'A × B',
    accent: 2,
    inputs: inN(2),
    outputs: [out('out', '×')],
  },
  {
    type: 'divide',
    name: 'Divide',
    category: 'Math',
    icon: Divide,
    blurb: 'A ÷ B',
    accent: 2,
    inputs: inN(2),
    outputs: [out('out', '÷')],
  },
  {
    type: 'scale',
    name: 'Scale + Offset',
    category: 'Math',
    icon: SlidersHorizontal,
    blurb: 'Linear transform y = mx + b',
    accent: 2,
    inputs: [out('in', 'x')],
    outputs: [out('out', 'y')],
    config: { label: 'Gain', value: '1.0' },
  },
  {
    type: 'const',
    name: 'Constant',
    category: 'Math',
    icon: Hash,
    blurb: 'Fixed setpoint',
    accent: 2,
    inputs: [],
    outputs: [out('out', 'k')],
    config: { label: 'Value', value: '230' },
  },
  // ---- Logic ------------------------------------------------------------
  {
    type: 'greater',
    name: 'Greater Than',
    category: 'Logic',
    icon: GitCompareArrows,
    blurb: 'A > setpoint',
    accent: 4,
    inputs: [out('in', 'A')],
    outputs: [out('out', 'q', 'bool')],
    config: { label: 'Setpoint', value: '250', suffix: 'V' },
  },
  {
    type: 'and',
    name: 'AND',
    category: 'Logic',
    icon: Binary,
    blurb: 'All inputs true',
    accent: 4,
    inputs: inN(2, 'bool'),
    outputs: [out('out', 'q', 'bool')],
  },
  {
    type: 'or',
    name: 'OR',
    category: 'Logic',
    icon: Split,
    blurb: 'Any input true',
    accent: 4,
    inputs: inN(2, 'bool'),
    outputs: [out('out', 'q', 'bool')],
  },
  {
    type: 'latch',
    name: 'Latch (SR)',
    category: 'Logic',
    icon: ArrowRightLeft,
    blurb: 'Set / reset flip-flop',
    accent: 4,
    inputs: [out('set', 'S', 'bool'), out('reset', 'R', 'bool')],
    outputs: [out('out', 'q', 'bool')],
  },
  {
    type: 'select',
    name: 'Select',
    category: 'Logic',
    icon: Workflow,
    blurb: 'Pick A or B on gate',
    accent: 4,
    inputs: [out('a', 'A'), out('b', 'B'), out('sel', 'sel', 'bool')],
    outputs: [out('out', 'y')],
  },
  // ---- Analytics --------------------------------------------------------
  {
    type: 'average',
    name: 'Rolling Average',
    category: 'Analytics',
    icon: Sigma,
    blurb: 'Windowed mean',
    accent: 1,
    inputs: [out('in', 'x')],
    outputs: [out('out', 'x̄')],
    config: { label: 'Window', value: '15', suffix: 'min' },
  },
  {
    type: 'minmax',
    name: 'Min / Max',
    category: 'Analytics',
    icon: Gauge,
    blurb: 'Track extremes',
    accent: 1,
    inputs: [out('in', 'x')],
    outputs: [out('min', 'min'), out('max', 'max')],
  },
  {
    type: 'integrate',
    name: 'Integrate',
    category: 'Analytics',
    icon: FunctionSquare,
    blurb: 'kW → kWh accumulation',
    accent: 1,
    inputs: [out('in', 'kW')],
    outputs: [out('out', 'kWh')],
  },
  {
    type: 'rate',
    name: 'Rate of Change',
    category: 'Analytics',
    icon: TrendingUp,
    blurb: 'dx/dt slope',
    accent: 1,
    inputs: [out('in', 'x')],
    outputs: [out('out', 'dx/dt')],
  },
  {
    type: 'deadband',
    name: 'Deadband',
    category: 'Analytics',
    icon: Filter,
    blurb: 'Ignore small changes',
    accent: 1,
    inputs: [out('in', 'x')],
    outputs: [out('out', 'y')],
    config: { label: 'Band', value: '2.0' },
  },
  // ---- Alarming ---------------------------------------------------------
  {
    type: 'threshold-alarm',
    name: 'Threshold Alarm',
    category: 'Alarming',
    icon: BellRing,
    blurb: 'Warn / critical ramp',
    accent: 5,
    inputs: [out('in', 'x')],
    outputs: [out('active', 'alarm', 'bool'), out('sev', 'sev', 'event')],
    config: { label: 'Critical ≥', value: '253', suffix: 'V' },
  },
  {
    type: 'delay',
    name: 'On-Delay',
    category: 'Alarming',
    icon: TimerReset,
    blurb: 'Debounce before firing',
    accent: 5,
    inputs: [out('in', 'q', 'bool')],
    outputs: [out('out', 'q', 'bool')],
    config: { label: 'For', value: '5', suffix: 'min' },
  },
  {
    type: 'schedule',
    name: 'Schedule Gate',
    category: 'Alarming',
    icon: CalendarClock,
    blurb: 'Only alarm in hours',
    accent: 5,
    inputs: [out('in', 'q', 'bool')],
    outputs: [out('out', 'q', 'bool')],
    config: { label: 'Window', value: 'Mon–Fri 8–18' },
  },
  {
    type: 'notify-email',
    name: 'Email Notify',
    category: 'Alarming',
    icon: Mail,
    blurb: 'Send alarm e-mail',
    accent: 5,
    inputs: [out('trig', 'trig', 'event')],
    outputs: [],
    config: { label: 'To', value: 'ops@acme.com' },
  },
  {
    type: 'notify-webhook',
    name: 'Webhook',
    category: 'Alarming',
    icon: Webhook,
    blurb: 'POST to external system',
    accent: 5,
    inputs: [out('trig', 'trig', 'event')],
    outputs: [],
    config: { label: 'URL', value: 'https://…/hook' },
  },
  {
    type: 'alarm-bell',
    name: 'Console Alarm',
    category: 'Alarming',
    icon: Bell,
    blurb: 'Raise to alarm console',
    accent: 5,
    inputs: [out('trig', 'trig', 'event')],
    outputs: [],
    config: { label: 'Priority', value: 'High' },
  },
  // ---- Reporting --------------------------------------------------------
  {
    type: 'report-kpi',
    name: 'KPI Tile',
    category: 'Reporting',
    icon: Activity,
    blurb: 'Publish a dashboard tile',
    accent: 3,
    inputs: [out('in', 'x')],
    outputs: [],
    config: { label: 'Label', value: 'Peak Demand' },
  },
  {
    type: 'report-roll',
    name: 'Rollup',
    category: 'Reporting',
    icon: Database,
    blurb: 'Aggregate to interval',
    accent: 3,
    inputs: [out('in', 'x')],
    outputs: [out('out', 'agg')],
    config: { label: 'Bucket', value: '1 h' },
  },
  {
    type: 'report-export',
    name: 'Report Export',
    category: 'Reporting',
    icon: FileBarChart,
    blurb: 'Feed scheduled report',
    accent: 3,
    inputs: inN(3, 'any'),
    outputs: [],
    config: { label: 'Schedule', value: 'Daily 06:00' },
  },
  {
    type: 'report-publish',
    name: 'Publish',
    category: 'Reporting',
    icon: Send,
    blurb: 'Emit to data console',
    accent: 3,
    inputs: [out('in', 'x')],
    outputs: [],
    config: { label: 'Series', value: 'derived.kpi' },
  },
]

export const BLOCK_BY_TYPE: Record<string, BlockSpec> = Object.fromEntries(
  BLOCKS.map((b) => [b.type, b])
)

export const CATEGORIES: KitCategory[] = [
  'Sources',
  'Math',
  'Logic',
  'Analytics',
  'Alarming',
  'Reporting',
]

/** Lucide icon per kit category, for palette section headers. */
export const CATEGORY_ICON: Record<KitCategory, LucideIcon> = {
  Sources: Zap,
  Math: Plus,
  Logic: Binary,
  Analytics: Sigma,
  Alarming: AlarmClock,
  Reporting: FileBarChart,
}

/** The chart-N accent token resolved to a CSS var reference. */
export const accentVar = (accent: number) => `var(--chart-${accent})`

/** Port colour by signal kind — bool/event read differently from analog. */
export const portColor = (kind: PortKind): string => {
  switch (kind) {
    case 'bool':
      return 'var(--chart-4)'
    case 'event':
      return 'var(--chart-5)'
    case 'any':
      return 'var(--muted-foreground)'
    default:
      return 'var(--chart-2)'
  }
}
