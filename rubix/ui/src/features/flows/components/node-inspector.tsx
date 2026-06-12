import { Settings2 } from 'lucide-react'
import type { Node } from '@xyflow/react'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import type { FlowNodeData } from './flow-node'

const KIND_META: Record<FlowNodeData['kind'], { label: string; tag: string }> = {
  in: { label: 'Source actor', tag: '#[subscribe]' },
  logic: { label: 'Logic actor', tag: '#[actor]' },
  out: { label: 'Write actor', tag: '#[command]' },
  agent: { label: 'AI actor', tag: '#[agent]' },
}

/** Right rail: details of the selected wiresheet node. */
export function NodeInspector({ node }: { node: Node<FlowNodeData> | undefined }) {
  if (!node) {
    return (
      <div className='text-muted-foreground grid h-full place-items-center px-4 text-center text-[12px]'>
        Select a node to inspect its ports and config.
      </div>
    )
  }
  const meta = KIND_META[node.data.kind]
  const ins = node.data.hasIn ? ['in'] : []
  const outs = node.data.hasOut ? ['out'] : []

  return (
    <div className='flex flex-col gap-4'>
      <div>
        <div className='text-[13.5px] font-semibold'>{node.data.title}</div>
        <div className='text-muted-foreground font-mono text-[10.5px]'>{node.data.sub}</div>
      </div>

      <div>
        <div className='eyebrow pb-1.5 text-[9.5px]'>Node type</div>
        <div className='flex items-center justify-between gap-2 text-[12px]'>
          <span className='font-medium'>{meta.label}</span>
          <Badge variant='outline' className='h-[18px] px-1.5 font-mono text-[10px]'>
            {meta.tag}
          </Badge>
        </div>
      </div>

      <div>
        <div className='eyebrow pb-1.5 text-[9.5px]'>Ports</div>
        <div className='space-y-1'>
          {[...ins.map((p) => [p, 'input'] as const), ...outs.map((p) => [p, 'output'] as const)].map(
            ([port, dir]) => (
              <div
                key={`${port}-${dir}`}
                className='border-border flex items-center justify-between rounded-md border px-2 py-1.5 text-[11.5px]'
              >
                <span className='flex items-center gap-2 font-mono'>
                  <span
                    className={
                      dir === 'input'
                        ? 'bg-muted-foreground size-1.5 rounded-full'
                        : 'bg-primary size-1.5 rounded-full'
                    }
                  />
                  {port}
                </span>
                <span className='text-muted-foreground'>{dir}</span>
              </div>
            )
          )}
        </div>
      </div>

      <Button variant='outline' size='sm' className='w-full'>
        <Settings2 className='size-3.5' /> Configure actor
      </Button>
    </div>
  )
}
