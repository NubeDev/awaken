/**
 * The right-hand inspector: properties of the currently selected node. For a kit
 * block it shows the block's ports + its (demo) tunable; for a live source it
 * shows the register's site/meter/quantity/unit. Editing the tunable is local and
 * illustrative — the value writes back into the node's data so the canvas re-reads
 * it, but nothing is evaluated. When nothing is selected it shows a short "how to"
 * so the empty state still teaches the surface.
 */
import { MousePointerClick } from 'lucide-react'
import type { Node } from '@xyflow/react'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Badge } from '@/components/ui/badge'
import { Separator } from '@/components/ui/separator'
import { accentVar, portColor, type BlockSpec, type PortSpec } from './palette'
import type { SourcePoint } from './source-nodes'

function PortRow({ port }: { port: PortSpec }) {
  return (
    <div className='flex items-center gap-2 py-0.5'>
      <span
        className='size-2 shrink-0 rounded-full'
        style={{ background: portColor(port.kind) }}
      />
      <span className='text-xs font-medium'>{port.label}</span>
      <span className='text-muted-foreground ml-auto font-mono text-[10px] uppercase'>
        {port.kind}
      </span>
    </div>
  )
}

export function Inspector({
  node,
  onConfigChange,
}: {
  node: Node | null
  onConfigChange: (nodeId: string, value: string) => void
}) {
  return (
    <div className='bg-muted/30 flex h-full w-72 shrink-0 flex-col border-l'>
      <div className='border-b px-3 py-2.5'>
        <div className='text-sm font-semibold'>Properties</div>
        <p className='text-muted-foreground text-[11px]'>
          {node ? 'Selected block' : 'Nothing selected'}
        </p>
      </div>

      {!node ? (
        <div className='text-muted-foreground flex flex-1 flex-col items-center justify-center gap-2 px-6 text-center'>
          <MousePointerClick className='size-6 opacity-50' />
          <p className='text-xs leading-relaxed'>
            Select a block to edit its parameters, or drag from the kit on the
            left to add logic. Connect a port to another to wire signal flow.
          </p>
        </div>
      ) : node.type === 'source' ? (
        <SourceInspector point={(node.data as { point: SourcePoint }).point} />
      ) : (
        <BlockInspector
          spec={(node.data as { spec: BlockSpec }).spec}
          nodeId={node.id}
          onConfigChange={onConfigChange}
        />
      )}
    </div>
  )
}

function field(label: string, value: string) {
  return (
    <div className='flex items-baseline justify-between gap-2 py-1'>
      <span className='text-muted-foreground text-xs'>{label}</span>
      <span className='truncate text-xs font-medium'>{value}</span>
    </div>
  )
}

function SourceInspector({ point }: { point: SourcePoint }) {
  return (
    <div className='space-y-3 px-3 py-3'>
      <div>
        <div className='text-sm font-semibold'>{point.name}</div>
        <Badge variant='secondary' className='mt-1 text-[10px]'>
          Live source
        </Badge>
      </div>
      <Separator />
      <div>
        {field('Site', point.siteName)}
        {field('Meter', point.meterName)}
        {field('Quantity', point.quantity)}
        {field('Unit', point.unit)}
      </div>
      <Separator />
      <p className='text-muted-foreground text-[11px] leading-relaxed'>
        This point streams from the live portfolio. Wire its output into the kit
        to build derived alarming or reporting logic.
      </p>
    </div>
  )
}

function BlockInspector({
  spec,
  nodeId,
  onConfigChange,
}: {
  spec: BlockSpec
  nodeId: string
  onConfigChange: (nodeId: string, value: string) => void
}) {
  const Icon = spec.icon
  const accent = accentVar(spec.accent)
  return (
    <div className='space-y-3 overflow-y-auto px-3 py-3'>
      <div className='flex items-center gap-2'>
        <span
          className='flex size-7 items-center justify-center rounded-md'
          style={{ background: `color-mix(in oklch, ${accent} 18%, transparent)` }}
        >
          <Icon className='size-4' style={{ color: accent }} />
        </span>
        <div>
          <div className='text-sm font-semibold'>{spec.name}</div>
          <div className='text-muted-foreground text-[11px]'>{spec.category}</div>
        </div>
      </div>

      <p className='text-muted-foreground text-[11px] leading-relaxed'>
        {spec.blurb}
      </p>

      {spec.config ? (
        <>
          <Separator />
          <div className='grid gap-1.5'>
            <Label className='text-xs'>{spec.config.label}</Label>
            <div className='relative'>
              <Input
                defaultValue={spec.config.value}
                onChange={(e) => onConfigChange(nodeId, e.target.value)}
                className='h-8 text-xs'
              />
              {spec.config.suffix ? (
                <span className='text-muted-foreground absolute top-1/2 right-2.5 -translate-y-1/2 text-[10px]'>
                  {spec.config.suffix}
                </span>
              ) : null}
            </div>
          </div>
        </>
      ) : null}

      {spec.inputs.length > 0 ? (
        <>
          <Separator />
          <div>
            <div className='text-muted-foreground mb-1 text-[10px] font-semibold uppercase'>
              Inputs
            </div>
            {spec.inputs.map((p) => (
              <PortRow key={p.id} port={p} />
            ))}
          </div>
        </>
      ) : null}

      {spec.outputs.length > 0 ? (
        <>
          <Separator />
          <div>
            <div className='text-muted-foreground mb-1 text-[10px] font-semibold uppercase'>
              Outputs
            </div>
            {spec.outputs.map((p) => (
              <PortRow key={p.id} port={p} />
            ))}
          </div>
        </>
      ) : null}
    </div>
  )
}
