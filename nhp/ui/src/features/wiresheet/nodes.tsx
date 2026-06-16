/**
 * The two React-Flow node renderers for the wiresheet: a function BLOCK (from the
 * kit, palette.ts) and a SOURCE point (a live meter register, source-nodes.ts).
 * Both are styled to read like a Sedona/Niagara wiresheet block — a titled header
 * with a tinted accent rail, a port column down each side, and a small body — but
 * they are pure presentation; no signal is evaluated.
 *
 * Handles use React-Flow's <Handle>. Inputs are targets on the left, outputs are
 * sources on the right; each handle is coloured by its port kind so analog / bool
 * / event wiring reads at a glance.
 */
import { memo } from 'react'
import { Handle, Position, type NodeProps } from '@xyflow/react'
import { Zap } from 'lucide-react'
import { cn } from '@/lib/utils'
import { accentVar, portColor, type BlockSpec, type PortSpec } from './palette'
import type { SourcePoint } from './source-nodes'

/** Shared port row — a coloured handle plus its label, mirrored by side. */
function Port({
  port,
  side,
  index,
  total,
}: {
  port: PortSpec
  side: 'in' | 'out'
  index: number
  total: number
}) {
  const isIn = side === 'in'
  // Spread handles evenly down the node's vertical edge.
  const top = `${((index + 1) / (total + 1)) * 100}%`
  return (
    <Handle
      id={port.id}
      type={isIn ? 'target' : 'source'}
      position={isIn ? Position.Left : Position.Right}
      style={{
        top,
        width: 9,
        height: 9,
        background: portColor(port.kind),
        border: '2px solid var(--card)',
        boxShadow: '0 0 0 1px var(--border)',
      }}
    >
      <span
        className={cn(
          'text-muted-foreground pointer-events-none absolute top-1/2 -translate-y-1/2 text-[9px] font-medium tracking-tight whitespace-nowrap',
          isIn ? 'left-3' : 'right-3'
        )}
      >
        {port.label}
      </span>
    </Handle>
  )
}

interface BlockNodeData {
  spec: BlockSpec
  [key: string]: unknown
}

export const BlockNode = memo(function BlockNode({
  data,
  selected,
}: NodeProps) {
  const { spec } = data as BlockNodeData
  const Icon = spec.icon
  const accent = accentVar(spec.accent)
  const rows = Math.max(spec.inputs.length, spec.outputs.length, 1)

  return (
    <div
      className={cn(
        'bg-card/95 supports-[backdrop-filter]:bg-card/80 group relative w-44 rounded-lg border shadow-sm backdrop-blur transition-all',
        selected
          ? 'ring-primary/60 border-primary/40 shadow-lg ring-2'
          : 'border-border hover:shadow-md'
      )}
    >
      {/* accent rail */}
      <div
        className='absolute inset-y-0 left-0 w-1 rounded-l-lg'
        style={{ background: accent }}
      />
      {/* header */}
      <div className='flex items-center gap-2 px-3 pt-2.5 pb-1.5'>
        <span
          className='flex size-6 shrink-0 items-center justify-center rounded-md'
          style={{ background: `color-mix(in oklch, ${accent} 18%, transparent)` }}
        >
          <Icon className='size-3.5' style={{ color: accent }} />
        </span>
        <div className='min-w-0'>
          <div className='truncate text-[13px] leading-tight font-semibold'>
            {spec.name}
          </div>
          <div className='text-muted-foreground truncate text-[10px] leading-tight'>
            {spec.category}
          </div>
        </div>
      </div>

      {/* body / config */}
      {spec.config ? (
        <div className='mx-3 mb-2 rounded-md border border-dashed px-2 py-1'>
          <div className='flex items-baseline justify-between gap-2'>
            <span className='text-muted-foreground text-[10px]'>
              {spec.config.label}
            </span>
            <span className='font-mono text-[11px] font-medium tabular-nums'>
              {spec.config.value}
              {spec.config.suffix ? (
                <span className='text-muted-foreground ml-0.5 text-[9px]'>
                  {spec.config.suffix}
                </span>
              ) : null}
            </span>
          </div>
        </div>
      ) : (
        <div className='text-muted-foreground px-3 pb-2 text-[10px] leading-snug'>
          {spec.blurb}
        </div>
      )}

      {/* port lane spacer so handles have vertical room */}
      <div style={{ height: rows * 14 }} className='relative'>
        {spec.inputs.map((p, i) => (
          <Port key={p.id} port={p} side='in' index={i} total={spec.inputs.length} />
        ))}
        {spec.outputs.map((p, i) => (
          <Port
            key={p.id}
            port={p}
            side='out'
            index={i}
            total={spec.outputs.length}
          />
        ))}
      </div>
    </div>
  )
})

interface SourceNodeData {
  point: SourcePoint
  [key: string]: unknown
}

export const SourceNode = memo(function SourceNode({
  data,
  selected,
}: NodeProps) {
  const { point } = data as SourceNodeData

  return (
    <div
      className={cn(
        'group relative w-48 overflow-hidden rounded-lg border shadow-sm transition-all',
        selected
          ? 'ring-primary/60 border-primary/40 shadow-lg ring-2'
          : 'border-border hover:shadow-md'
      )}
      style={{
        background:
          'linear-gradient(135deg, color-mix(in oklch, var(--chart-2) 12%, var(--card)) 0%, var(--card) 60%)',
      }}
    >
      <div className='flex items-center gap-2 px-3 pt-2.5 pb-1'>
        <span
          className='flex size-6 shrink-0 items-center justify-center rounded-md'
          style={{ background: 'color-mix(in oklch, var(--chart-2) 22%, transparent)' }}
        >
          <Zap className='size-3.5' style={{ color: 'var(--chart-2)' }} />
        </span>
        <div className='min-w-0'>
          <div className='truncate text-[13px] leading-tight font-semibold'>
            {point.name}
          </div>
          <div className='text-muted-foreground truncate text-[10px] leading-tight'>
            {point.siteName} · {point.meterName}
          </div>
        </div>
      </div>

      <div className='flex items-center justify-between px-3 pt-0.5 pb-2.5'>
        <span className='bg-muted/60 text-muted-foreground rounded px-1.5 py-0.5 font-mono text-[9px] tracking-tight uppercase'>
          {point.quantity}
        </span>
        <span className='font-mono text-[11px] font-semibold tabular-nums'>
          live<span className='text-muted-foreground ml-1'>{point.unit}</span>
        </span>
      </div>

      <Handle
        id='out'
        type='source'
        position={Position.Right}
        style={{
          top: '50%',
          width: 10,
          height: 10,
          background: 'var(--chart-2)',
          border: '2px solid var(--card)',
          boxShadow: '0 0 0 1px var(--border)',
        }}
      />
    </div>
  )
})

