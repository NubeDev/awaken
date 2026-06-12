import { Handle, Position, type NodeProps } from '@xyflow/react'
import {
  Calculator,
  Calendar,
  Droplet,
  Route,
  Sparkles,
  Thermometer,
  type LucideIcon,
} from 'lucide-react'
import { cn } from '@/lib/utils'

const ICONS: Record<string, LucideIcon> = {
  thermometer: Thermometer,
  calendar: Calendar,
  function: Calculator,
  route: Route,
  droplet: Droplet,
  sparkles: Sparkles,
}

const KIND_RING: Record<string, string> = {
  in: 'text-chart-2',
  logic: 'text-chart-1',
  out: 'text-chart-4',
  agent: 'text-primary',
}

export type FlowNodeData = {
  title: string
  sub: string
  icon: string
  kind: 'in' | 'logic' | 'out' | 'agent'
  hasIn?: boolean
  hasOut?: boolean
}

/** A wiresheet block: titled card with typed in/out handles, styled by kind. */
export function FlowNode({ data }: NodeProps & { data: FlowNodeData }) {
  const Icon = ICONS[data.icon] ?? Route
  return (
    <div className='bg-card min-w-[170px] rounded-lg border border-border shadow-md'>
      {data.hasIn && <Handle type='target' position={Position.Left} className='!bg-border-strong' />}
      <div className='flex items-center gap-2.5 px-3 py-2.5'>
        <span
          className={cn(
            'bg-accent grid size-8 shrink-0 place-items-center rounded-md',
            KIND_RING[data.kind]
          )}
        >
          <Icon className='size-4' />
        </span>
        <div className='min-w-0'>
          <div className='truncate text-[12.5px] font-semibold'>{data.title}</div>
          <div className='text-muted-foreground truncate font-mono text-[10.5px]'>{data.sub}</div>
        </div>
      </div>
      {data.hasOut && (
        <Handle type='source' position={Position.Right} className='!bg-primary' />
      )}
    </div>
  )
}
