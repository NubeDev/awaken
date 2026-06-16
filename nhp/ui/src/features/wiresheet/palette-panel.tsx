/**
 * The left-hand kit palette: collapsible categories of draggable function blocks
 * (palette.ts) plus a "Sources" section built from the live portfolio
 * (source-nodes.ts). Dragging an item sets a dnd payload the canvas reads on drop
 * to instantiate a node at the cursor. A search box filters across both kits.
 *
 * Pure presentation + native HTML5 drag — no React-Flow state here; the canvas
 * owns the graph and just consumes the drop.
 */
import { useMemo, useState } from 'react'
import { ChevronRight, Search } from 'lucide-react'
import { cn } from '@/lib/utils'
import { Input } from '@/components/ui/input'
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from '@/components/ui/collapsible'
import { ScrollArea } from '@/components/ui/scroll-area'
import {
  BLOCKS,
  CATEGORIES,
  CATEGORY_ICON,
  accentVar,
  type BlockSpec,
  type KitCategory,
} from './palette'
import type { SourceGroup, SourcePoint } from './source-nodes'

/** dnd payload format the canvas listens for. */
export const DND_MIME = 'application/nhp-wiresheet'

export interface DragPayload {
  kind: 'block' | 'source'
  type: string
}

function startDrag(e: React.DragEvent, payload: DragPayload) {
  e.dataTransfer.setData(DND_MIME, JSON.stringify(payload))
  e.dataTransfer.effectAllowed = 'move'
}

function BlockChip({ spec }: { spec: BlockSpec }) {
  const Icon = spec.icon
  const accent = accentVar(spec.accent)
  return (
    <div
      draggable
      onDragStart={(e) => startDrag(e, { kind: 'block', type: spec.type })}
      className='border-border bg-card hover:border-primary/40 hover:bg-accent/50 group flex cursor-grab items-center gap-2 rounded-md border px-2 py-1.5 text-left transition-colors active:cursor-grabbing'
    >
      <span
        className='flex size-6 shrink-0 items-center justify-center rounded'
        style={{ background: `color-mix(in oklch, ${accent} 16%, transparent)` }}
      >
        <Icon className='size-3.5' style={{ color: accent }} />
      </span>
      <div className='min-w-0'>
        <div className='truncate text-xs font-medium leading-tight'>
          {spec.name}
        </div>
        <div className='text-muted-foreground truncate text-[10px] leading-tight'>
          {spec.blurb}
        </div>
      </div>
    </div>
  )
}

function SourceChip({ point }: { point: SourcePoint }) {
  return (
    <div
      draggable
      onDragStart={(e) => startDrag(e, { kind: 'source', type: point.type })}
      className='border-border bg-card hover:border-primary/40 hover:bg-accent/50 flex cursor-grab items-center justify-between gap-2 rounded-md border px-2 py-1 text-left transition-colors active:cursor-grabbing'
    >
      <span className='truncate text-xs font-medium'>{point.name}</span>
      <span className='text-muted-foreground shrink-0 font-mono text-[9px] uppercase'>
        {point.unit}
      </span>
    </div>
  )
}

function Section({
  title,
  icon: Icon,
  defaultOpen,
  count,
  children,
}: {
  title: string
  icon: React.ComponentType<{ className?: string }>
  defaultOpen?: boolean
  count: number
  children: React.ReactNode
}) {
  const [open, setOpen] = useState(defaultOpen ?? false)
  return (
    <Collapsible open={open} onOpenChange={setOpen}>
      <CollapsibleTrigger className='hover:bg-accent/60 flex w-full items-center gap-1.5 rounded-md px-2 py-1.5 text-left'>
        <ChevronRight
          className={cn(
            'text-muted-foreground size-3.5 transition-transform',
            open && 'rotate-90'
          )}
        />
        <Icon className='text-muted-foreground size-3.5' />
        <span className='text-xs font-semibold'>{title}</span>
        <span className='text-muted-foreground ml-auto text-[10px] tabular-nums'>
          {count}
        </span>
      </CollapsibleTrigger>
      <CollapsibleContent className='space-y-1 px-2 pt-1 pb-2'>
        {children}
      </CollapsibleContent>
    </Collapsible>
  )
}

export function PalettePanel({
  sources,
  isLoading,
}: {
  sources: SourceGroup[]
  isLoading: boolean
}) {
  const [q, setQ] = useState('')
  const query = q.trim().toLowerCase()

  const blocksByCat = useMemo(() => {
    const map = new Map<KitCategory, BlockSpec[]>()
    for (const b of BLOCKS) {
      if (
        query &&
        !`${b.name} ${b.blurb} ${b.category}`.toLowerCase().includes(query)
      )
        continue
      if (!map.has(b.category)) map.set(b.category, [])
      map.get(b.category)!.push(b)
    }
    return map
  }, [query])

  const filteredSources = useMemo(() => {
    if (!query) return sources
    return sources
      .map((g) => ({
        ...g,
        meters: g.meters
          .map((m) => ({
            ...m,
            points: m.points.filter((p) =>
              `${p.name} ${p.meterName} ${p.quantity}`
                .toLowerCase()
                .includes(query)
            ),
          }))
          .filter((m) => m.points.length),
      }))
      .filter((g) => g.meters.length)
  }, [sources, query])

  const sourceCount = filteredSources.reduce(
    (n, g) => n + g.meters.reduce((m, x) => m + x.points.length, 0),
    0
  )

  return (
    <div className='bg-muted/30 flex h-full w-64 shrink-0 flex-col border-r'>
      <div className='border-b px-3 py-2.5'>
        <div className='text-sm font-semibold'>Block Kit</div>
        <p className='text-muted-foreground text-[11px]'>
          Drag blocks onto the sheet
        </p>
        <div className='relative mt-2'>
          <Search className='text-muted-foreground absolute top-1/2 left-2 size-3.5 -translate-y-1/2' />
          <Input
            value={q}
            onChange={(e) => setQ(e.target.value)}
            placeholder='Search blocks…'
            className='h-8 pl-7 text-xs'
          />
        </div>
      </div>

      <ScrollArea className='min-h-0 flex-1'>
        <div className='py-1'>
          {/* Live sources first — the real data. */}
          <Section
            title='Sources'
            icon={CATEGORY_ICON.Sources}
            count={sourceCount}
            defaultOpen={!!query}
          >
            {isLoading ? (
              <div className='text-muted-foreground px-1 py-2 text-[11px]'>
                Loading live points…
              </div>
            ) : filteredSources.length === 0 ? (
              <div className='text-muted-foreground px-1 py-2 text-[11px]'>
                No matching points
              </div>
            ) : (
              filteredSources.map((g) => (
                <div key={g.siteName} className='mb-1.5'>
                  <div className='text-muted-foreground px-1 py-0.5 text-[10px] font-semibold tracking-wide uppercase'>
                    {g.siteName}
                  </div>
                  {g.meters.map((m) => (
                    <div key={m.meterName} className='mb-1'>
                      <div className='text-muted-foreground/80 px-1 text-[10px]'>
                        {m.meterName}
                      </div>
                      <div className='mt-0.5 space-y-0.5'>
                        {m.points.map((p) => (
                          <SourceChip key={p.registerId} point={p} />
                        ))}
                      </div>
                    </div>
                  ))}
                </div>
              ))
            )}
          </Section>

          {CATEGORIES.filter((c) => c !== 'Sources').map((cat) => {
            const items = blocksByCat.get(cat) ?? []
            if (items.length === 0) return null
            return (
              <Section
                key={cat}
                title={cat}
                icon={CATEGORY_ICON[cat]}
                count={items.length}
                defaultOpen={!!query}
              >
                {items.map((b) => (
                  <BlockChip key={b.type} spec={b} />
                ))}
              </Section>
            )
          })}
        </div>
      </ScrollArea>
    </div>
  )
}
