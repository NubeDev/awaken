import { Trash2 } from 'lucide-react'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import type { RunBoardResponse } from '@/api/types'
import type { FlowNodeData } from './flow-node'
import { NodeConfigForm } from './node-config-form'

const KIND_LABEL: Record<FlowNodeData['schema']['kind'], string> = {
  source: 'Source',
  logic: 'Logic',
  sink: 'Sink',
  agent: 'AI',
}

type NodeInspectorProps = {
  /** The selected node's data, or undefined when nothing is selected. */
  node: FlowNodeData | undefined
  /** Patch a config key on the selected node. */
  onConfigChange: (key: string, value: unknown) => void
  /** Remove the selected node and its wires. */
  onDelete: () => void
  /** Last test-run outputs, so each outport can show the value it produced. */
  lastRun?: RunBoardResponse
  /** The board's editing scope, used to resolve dropdown (`option_source`) fields. */
  scope?: { org?: string; site?: string }
}

/**
 * Right rail: the selected node's identity, ports, and a config form rendered
 * from the component schema. Editing a field patches the node's `config` so the
 * change is part of the next save — nothing here is hardcoded per component.
 * When a test run exists, each outport shows the value it emitted.
 */
export function NodeInspector({
  node,
  onConfigChange,
  onDelete,
  lastRun,
  scope,
}: NodeInspectorProps) {
  if (!node) {
    return (
      <div className='text-muted-foreground grid h-full place-items-center px-4 text-center text-[12px]'>
        Select a node to edit its config, or drag one in from the palette.
      </div>
    )
  }
  const { schema } = node

  // This node's emitted values from the last run, keyed by outport id.
  const portValues = new Map(
    (lastRun?.outputs ?? [])
      .filter((o) => o.node === node.nodeId)
      .map((o) => [o.port, o.value] as const)
  )

  return (
    <div className='flex flex-col gap-4'>
      <div>
        <div className='flex items-center justify-between gap-2'>
          <div className='text-[13.5px] font-semibold'>{node.nodeId}</div>
          <Badge variant='outline' className='h-[18px] px-1.5 text-[10px]'>
            {KIND_LABEL[schema.kind]}
          </Badge>
        </div>
        <div className='text-muted-foreground font-mono text-[10.5px]'>{node.component}</div>
        {schema.description && (
          <p className='text-muted-foreground mt-1 text-[11px]'>{schema.description}</p>
        )}
      </div>

      <div>
        <div className='eyebrow pb-1.5 text-[9.5px]'>Config</div>
        <NodeConfigForm
          fields={schema.config}
          config={node.config}
          onChange={onConfigChange}
          scope={scope}
        />
      </div>

      <div>
        <div className='eyebrow pb-1.5 text-[9.5px]'>Ports</div>
        <div className='space-y-1'>
          {schema.inports.map((p) => (
            <PortRow key={`in-${p.id}`} id={p.id} dir='input' />
          ))}
          {schema.outports.map((p) => (
            <PortRow
              key={`out-${p.id}`}
              id={p.id}
              dir='output'
              value={portValues.has(p.id) ? portValues.get(p.id) : undefined}
              hasValue={portValues.has(p.id)}
            />
          ))}
        </div>
      </div>

      <Button variant='outline' size='sm' className='w-full' onClick={onDelete}>
        <Trash2 className='size-3.5' /> Delete node
      </Button>
    </div>
  )
}

function PortRow({
  id,
  dir,
  value,
  hasValue,
}: {
  id: string
  dir: 'input' | 'output'
  /** Last-run value emitted on this outport, if any. */
  value?: unknown
  hasValue?: boolean
}) {
  return (
    <div className='border-border rounded-md border px-2 py-1.5 text-[11.5px]'>
      <div className='flex items-center justify-between'>
        <span className='flex items-center gap-2 font-mono'>
          <span
            className={
              dir === 'input'
                ? 'bg-muted-foreground size-1.5 rounded-full'
                : id === 'error'
                  ? 'bg-destructive size-1.5 rounded-full'
                  : 'bg-primary size-1.5 rounded-full'
            }
          />
          {id}
        </span>
        <span className='text-muted-foreground'>{dir}</span>
      </div>
      {hasValue && (
        <div className='text-foreground/80 mt-1 truncate font-mono text-[10.5px]'>
          = {JSON.stringify(value)}
        </div>
      )}
    </div>
  )
}
