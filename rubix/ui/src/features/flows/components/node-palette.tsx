import {
  CalendarClock,
  CircleDot,
  Clock,
  Droplet,
  GitCompare,
  History,
  Calculator,
  Pin,
  Route,
  ShieldCheck,
  Sigma,
  Sparkles,
  Zap,
  type LucideIcon,
} from 'lucide-react'

type PaletteItem = { icon: LucideIcon; label: string }
type PaletteGroup = { group: string; items: PaletteItem[] }

/** Node catalogue for the wiresheet, grouped the way operators think. */
const PALETTE: PaletteGroup[] = [
  {
    group: 'Sources',
    items: [
      { icon: CircleDot, label: 'Point Read' },
      { icon: History, label: 'History Query' },
      { icon: CalendarClock, label: 'Schedule' },
      { icon: Clock, label: 'Interval' },
    ],
  },
  {
    group: 'Logic',
    items: [
      { icon: Calculator, label: 'Reset Curve' },
      { icon: Route, label: 'PID' },
      { icon: Sigma, label: 'Math' },
      { icon: GitCompare, label: 'Compare' },
    ],
  },
  {
    group: 'AI',
    items: [
      { icon: Sparkles, label: 'Agent Call' },
      { icon: ShieldCheck, label: 'HITL Gate' },
    ],
  },
  {
    group: 'Sinks',
    items: [
      { icon: Droplet, label: 'Point Write' },
      { icon: Zap, label: 'Emit Finding' },
      { icon: Pin, label: 'Pin Widget' },
    ],
  },
]

/** Left rail: draggable-looking node catalogue (wiring lands with stored boards). */
export function NodePalette() {
  return (
    <div className='flex flex-col gap-3'>
      {PALETTE.map((g) => (
        <div key={g.group}>
          <div className='eyebrow px-1 pb-1.5 text-[9.5px]'>{g.group}</div>
          <div className='flex flex-col gap-1'>
            {g.items.map(({ icon: Icon, label }) => (
              <div
                key={label}
                className='border-border bg-card hover:bg-accent hover:border-border-strong flex cursor-grab items-center gap-2.5 rounded-lg border px-2.5 py-2 text-[12px] font-medium transition-colors'
              >
                <Icon className='text-muted-foreground size-3.5' />
                {label}
              </div>
            ))}
          </div>
        </div>
      ))}
    </div>
  )
}
