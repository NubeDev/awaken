/**
 * Logic Studio — the wiresheet editor page (sidebar → Configure → Logic Studio).
 * A Sedona/Niagara-style programmable wiresheet for layering EXTRA EMS logic over
 * the live portfolio: custom alarming and custom reporting built by wiring live
 * meter points (left palette → "Sources", from the real records API) through a kit
 * of function blocks (math / logic / analytics / alarming / reporting).
 *
 * Layout: palette (left) · canvas (centre) · inspector (right), under a slim
 * toolbar. This is a FRONTEND-ONLY demo: the graph isn't persisted or evaluated —
 * it exists to show the shape of the feature. Live data is read once via the
 * reporting portfolio hook and offered as draggable source points.
 */
import { useMemo, useRef, useState } from 'react'
import { ReactFlowProvider, type Node } from '@xyflow/react'
import { Eraser, Play, Save, Workflow } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { toast } from 'sonner'
import { usePortfolio } from '@/features/reporting/use-portfolio'
import { PalettePanel } from './palette-panel'
import {
  WiresheetCanvas,
  type CanvasHandle,
  type SourceLookup,
} from './wiresheet-canvas'
import { Inspector } from './inspector'
import { buildSourceGroups, type SourcePoint } from './source-nodes'

export function WiresheetPage() {
  const { index, isLoading } = usePortfolio()
  const [selected, setSelected] = useState<Node | null>(null)
  const handleRef = useRef<CanvasHandle | null>(null)

  const groups = useMemo(() => buildSourceGroups(index), [index])

  // type → point lookup the canvas uses to instantiate a dropped source.
  const lookup = useMemo<SourceLookup>(() => {
    const map = new Map<string, SourcePoint>()
    for (const g of groups)
      for (const m of g.meters) for (const p of m.points) map.set(p.type, p)
    return map
  }, [groups])

  return (
    <div className='flex h-[calc(100svh-3.5rem)] flex-col'>
      {/* toolbar */}
      <div className='flex items-center gap-3 border-b px-4 py-2.5'>
        <div className='flex items-center gap-2'>
          <span className='bg-primary/10 flex size-8 items-center justify-center rounded-lg'>
            <Workflow className='text-primary size-4.5' />
          </span>
          <div>
            <div className='flex items-center gap-2'>
              <h1 className='text-sm font-semibold'>Logic Studio</h1>
              <Badge variant='secondary' className='text-[10px]'>
                Wiresheet
              </Badge>
            </div>
            <p className='text-muted-foreground text-[11px] leading-tight'>
              Build custom alarming & reporting logic over your live portfolio
            </p>
          </div>
        </div>

        <div className='ml-auto flex items-center gap-2'>
          <Button
            variant='ghost'
            size='sm'
            className='h-8'
            onClick={() => {
              handleRef.current?.clear()
              toast.message('Sheet cleared')
            }}
          >
            <Eraser className='mr-1 size-3.5' /> Clear
          </Button>
          <Button
            variant='outline'
            size='sm'
            className='h-8'
            onClick={() => toast.success('Sheet saved', {
              description: 'Demo only — not persisted to the backend.',
            })}
          >
            <Save className='mr-1 size-3.5' /> Save
          </Button>
          <Button
            size='sm'
            className='h-8'
            onClick={() => toast.success('Logic deployed', {
              description: 'Demo only — this sheet is illustrative.',
            })}
          >
            <Play className='mr-1 size-3.5' /> Deploy
          </Button>
        </div>
      </div>

      {/* studio body */}
      <div className='flex min-h-0 flex-1'>
        <PalettePanel sources={groups} isLoading={isLoading} />

        <div className='relative min-w-0 flex-1'>
          <ReactFlowProvider>
            <WiresheetCanvas
              sources={lookup}
              onSelect={setSelected}
              registerHandle={(h) => (handleRef.current = h)}
            />
          </ReactFlowProvider>
        </div>

        <Inspector
          node={selected}
          onConfigChange={(id, value) => handleRef.current?.setConfig(id, value)}
        />
      </div>
    </div>
  )
}
