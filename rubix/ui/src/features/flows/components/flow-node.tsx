import { Handle, Position, type NodeProps } from '@xyflow/react'
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
import type { ComponentView, PortView } from '@/api/types'
import { cn } from '@/lib/utils'

/** Component name → wiresheet icon. Falls back to a generic block. */
const ICONS: Record<string, LucideIcon> = {
  read_point: CircleDot,
  query_his: Calculator,
  trigger: Timer,
  agent_call: Sparkles,
  write_point: Droplet,
  emit_spark: Zap,
}

const KIND_RING: Record<ComponentView['kind'], string> = {
  source: 'text-chart-2',
  logic: 'text-chart-1',
  sink: 'text-chart-4',
  agent: 'text-primary',
}

/** Dot colour per port type, so a wire's class reads at a glance. */
const PORT_DOT: Record<PortView['port_type'], string> = {
  flow: '!bg-border-strong',
  scalar: '!bg-primary',
  object: '!bg-chart-1',
  error: '!bg-destructive',
}

/**
 * What a wiresheet block carries: the stored `id`, its `component` name and live
 * `config` map (both round-trip to the board graph), plus the component
 * `schema` so the node renders its real typed ports as labelled, connectable
 * handles.
 */
export type FlowNodeData = {
  nodeId: string
  component: string
  config: Record<string, unknown>
  schema: ComponentView
  /** Last test-run values for this node's outports, keyed by port id. */
  lastValues?: Record<string, unknown>
}

/** Render a port value compactly: strings quoted, objects as JSON, all clamped. */
function formatPortValue(value: unknown): string {
  const text = typeof value === 'string' ? value : JSON.stringify(value)
  if (text === undefined) return ''
  return text.length > 28 ? `${text.slice(0, 27)}…` : text
}

/**
 * A wiresheet block: a titled header above a two-column port body. Each port is
 * a labelled row whose handle sits on the node edge, so the card grows to fit
 * its ports (no absolute offsets) and every port shows its name.
 */
export function FlowNode({ data, selected }: NodeProps & { data: FlowNodeData }) {
  const { schema } = data
  const Icon = ICONS[data.component] ?? Route
  const rows = Math.max(schema.inports.length, schema.outports.length)

  return (
    <div
      className={cn(
        'bg-card w-[200px] rounded-lg border shadow-md',
        selected ? 'border-primary' : 'border-border'
      )}
    >
      <div className='border-border flex items-center gap-2.5 border-b px-3 py-2.5'>
        <span
          className={cn(
            'bg-accent grid size-8 shrink-0 place-items-center rounded-md',
            KIND_RING[schema.kind]
          )}
        >
          <Icon className='size-4' />
        </span>
        <div className='min-w-0'>
          <div className='truncate text-[12.5px] font-semibold'>{data.nodeId}</div>
          <div className='text-muted-foreground truncate font-mono text-[10.5px]'>
            {data.component}
          </div>
        </div>
      </div>

      <div className='grid grid-cols-2 gap-x-2 py-1.5'>
        <div className='flex flex-col'>
          {schema.inports.map((port) => (
            <PortRow key={`in-${port.id}`} port={port} side='in' />
          ))}
        </div>
        <div className='flex flex-col'>
          {schema.outports.map((port) => (
            <PortRow
              key={`out-${port.id}`}
              port={port}
              side='out'
              value={data.lastValues?.[port.id]}
              hasValue={data.lastValues ? port.id in data.lastValues : false}
            />
          ))}
        </div>
        {/* Reserve at least one row's height so a port-less side still lays out. */}
        {rows === 0 && <div className='h-[22px]' />}
      </div>
    </div>
  )
}

/**
 * One labelled port row; the handle is anchored to the node edge it belongs to.
 * An outport also shows the value it produced on the last test run, truncated
 * with the full value on hover.
 */
function PortRow({
  port,
  side,
  value,
  hasValue,
}: {
  port: PortView
  side: 'in' | 'out'
  value?: unknown
  hasValue?: boolean
}) {
  const dot = PORT_DOT[port.port_type]
  return (
    <div
      className={cn(
        'relative px-3 py-[3px] text-[10.5px]',
        side === 'out' && 'text-right'
      )}
    >
      <Handle
        id={port.id}
        type={side === 'in' ? 'target' : 'source'}
        position={side === 'in' ? Position.Left : Position.Right}
        className={cn('!top-[9px]', dot)}
        style={side === 'in' ? { left: -5 } : { right: -5 }}
      />
      <span className='text-muted-foreground block truncate font-mono'>{port.label}</span>
      {hasValue && (
        <span
          className='text-foreground/80 block truncate font-mono text-[10px]'
          title={typeof value === 'string' ? value : JSON.stringify(value)}
        >
          = {formatPortValue(value)}
        </span>
      )}
    </div>
  )
}
