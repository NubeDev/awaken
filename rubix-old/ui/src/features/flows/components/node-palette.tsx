import {
  Calculator,
  CircleDot,
  Droplet,
  Route,
  Sparkles,
  Timer,
  Zap,
  type LucideIcon,
} from 'lucide-react'
import type { ComponentView } from '@/api/types'

/** Component name → palette icon (mirrors the canvas node icons). */
const ICONS: Record<string, LucideIcon> = {
  read_point: CircleDot,
  query_his: Calculator,
  trigger: Timer,
  agent_call: Sparkles,
  write_point: Droplet,
  emit_spark: Zap,
}

/** MIME-ish key for the drag payload (the component name being dropped). */
export const DRAG_TYPE = 'application/rubix-component'

const GROUP_ORDER: ComponentView['kind'][] = ['source', 'logic', 'agent', 'sink']
const GROUP_LABEL: Record<ComponentView['kind'], string> = {
  source: 'Sources',
  logic: 'Logic',
  agent: 'AI',
  sink: 'Sinks',
}

type NodePaletteProps = {
  /** Catalogue from `GET /boards/components`; drives the entire palette. */
  components: ComponentView[]
}

/**
 * Left rail: the live component catalogue, grouped by kind. Each item is a drag
 * source — dragging it onto the canvas adds a node of that component (see the
 * canvas drop handler). The list is whatever the backend reports, so a new
 * component appears here with no client change.
 */
export function NodePalette({ components }: NodePaletteProps) {
  if (components.length === 0) {
    return <p className='text-muted-foreground text-[12px]'>Loading components…</p>
  }

  const byKind = (kind: ComponentView['kind']) =>
    components.filter((c) => c.kind === kind)

  return (
    <div className='flex flex-col gap-3'>
      {GROUP_ORDER.filter((k) => byKind(k).length > 0).map((kind) => (
        <div key={kind}>
          <div className='eyebrow px-1 pb-1.5 text-[9.5px]'>{GROUP_LABEL[kind]}</div>
          <div className='flex flex-col gap-1'>
            {byKind(kind).map((c) => (
              <PaletteItem key={c.component} component={c} />
            ))}
          </div>
        </div>
      ))}
    </div>
  )
}

function PaletteItem({ component }: { component: ComponentView }) {
  const Icon = ICONS[component.component] ?? Route
  return (
    <div
      draggable
      onDragStart={(e) => {
        e.dataTransfer.setData(DRAG_TYPE, component.component)
        e.dataTransfer.effectAllowed = 'move'
      }}
      title={component.description}
      className='border-border bg-card hover:bg-accent hover:border-border-strong flex cursor-grab items-center gap-2.5 rounded-lg border px-2.5 py-2 text-[12px] font-medium transition-colors'
    >
      <Icon className='text-muted-foreground size-3.5' />
      {component.label}
    </div>
  )
}
