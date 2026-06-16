// The builder's left rail: a palette of draggable tiles. Presets are grouped by
// surface (Records / Audit / Traces); saved charts (authored in the Query console)
// follow. Each card is HTML5-draggable — drop it on the grid to add a panel — and
// also click-to-add for keyboard/no-drag users. Already-placed charts are hidden.

import { GripVertical, Plus } from 'lucide-react'
import type { SavedChart } from '../../api/charts'
import { CHART_PRESETS, type ChartPreset, type PresetGroup } from './chart-presets'
import { PALETTE_DND_TYPE, chartIcon, encodeDrag, type PaletteDrag } from './board-palette'

const PRESET_GROUPS: PresetGroup[] = ['Records', 'Audit', 'Traces']

interface BoardPaletteProps {
  /** Saved charts not already on the board. */
  charts: SavedChart[]
  /** Add by click (drag handles the drop path itself). */
  onAdd: (drag: PaletteDrag) => void
}

export function BoardPalette({ charts, onAdd }: BoardPaletteProps) {
  return (
    <div className="flex h-full flex-col gap-4 overflow-y-auto rounded-xl border border-border bg-card/40 p-3">
      <div>
        <div className="mb-1.5 text-[10px] font-medium uppercase tracking-wide text-muted-foreground">
          Presets
        </div>
        <div className="space-y-3">
          {PRESET_GROUPS.map((group) => {
            const items = CHART_PRESETS.filter((p) => p.group === group)
            if (items.length === 0) return null
            return (
              <div key={group}>
                <div className="mb-1 px-0.5 text-[10px] font-medium text-muted-foreground/70">{group}</div>
                <div className="space-y-1.5">
                  {items.map((p) => (
                    <PresetCard key={p.name} preset={p} onAdd={onAdd} />
                  ))}
                </div>
              </div>
            )
          })}
        </div>
      </div>

      {charts.length > 0 && (
        <div>
          <div className="mb-1.5 text-[10px] font-medium uppercase tracking-wide text-muted-foreground">
            Saved charts
          </div>
          <div className="space-y-1.5">
            {charts.map((c) => (
              <ChartCard key={c.id} chart={c} onAdd={onAdd} />
            ))}
          </div>
        </div>
      )}
    </div>
  )
}

function Card({
  drag,
  onAdd,
  icon: Icon,
  title,
  subtitle,
}: {
  drag: PaletteDrag
  onAdd: (drag: PaletteDrag) => void
  icon: typeof GripVertical
  title: string
  subtitle?: string
}) {
  return (
    <button
      type="button"
      draggable
      onDragStart={(e) => {
        e.dataTransfer.setData(PALETTE_DND_TYPE, encodeDrag(drag))
        e.dataTransfer.effectAllowed = 'copy'
      }}
      onClick={() => onAdd(drag)}
      title="Drag onto the board, or click to add"
      className="group flex w-full cursor-grab items-center gap-2 rounded-lg border border-border bg-background px-2.5 py-2 text-left transition-colors hover:border-primary/40 hover:bg-accent active:cursor-grabbing"
    >
      <GripVertical size={13} className="shrink-0 text-muted-foreground/40 group-hover:text-muted-foreground" />
      <span className="grid size-6 shrink-0 place-items-center rounded-md bg-accent text-primary">
        <Icon size={13} />
      </span>
      <span className="min-w-0 flex-1">
        <span className="block truncate text-[12px] font-medium">{title}</span>
        {subtitle && <span className="block truncate text-[10.5px] text-muted-foreground">{subtitle}</span>}
      </span>
      <Plus size={13} className="shrink-0 text-muted-foreground opacity-0 transition-opacity group-hover:opacity-100" />
    </button>
  )
}

function PresetCard({ preset, onAdd }: { preset: ChartPreset; onAdd: (d: PaletteDrag) => void }) {
  return (
    <Card
      drag={{ source: 'preset', preset: preset.name }}
      onAdd={onAdd}
      icon={chartIcon(preset.config)}
      title={preset.name}
    />
  )
}

function ChartCard({ chart, onAdd }: { chart: SavedChart; onAdd: (d: PaletteDrag) => void }) {
  return (
    <Card
      drag={{ source: 'chart', chartId: chart.id }}
      onAdd={onAdd}
      icon={chartIcon(chart.config)}
      title={chart.name}
      subtitle="saved chart"
    />
  )
}
