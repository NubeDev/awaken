import { Plus } from 'lucide-react'
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip'
import { cn } from '@/lib/utils'
import { PALETTE, type PaletteEntry } from '../lib/palette'

type WidgetPaletteProps = {
  onPick: (entry: Extract<PaletteEntry, { available: true }>) => void
}

/** Left rail of pinnable tile kinds. Unavailable mockup kinds are disabled. */
export function WidgetPalette({ onPick }: WidgetPaletteProps) {
  return (
    <div className='space-y-2.5'>
      <div className='eyebrow text-[9.5px]'>Widgets</div>
      <div className='space-y-1.5'>
        {PALETTE.map((entry) =>
          entry.available ? (
            <button
              key={entry.label}
              onClick={() => onPick(entry)}
              className='hover:bg-accent group flex w-full items-start gap-2.5 rounded-lg border border-border px-2.5 py-2 text-left transition-colors'
            >
              <PaletteIcon entry={entry} />
              <div className='min-w-0 flex-1'>
                <div className='flex items-center gap-1 text-[12.5px] font-medium'>
                  {entry.label}
                  <Plus className='text-muted-foreground size-3 opacity-0 transition-opacity group-hover:opacity-100' />
                </div>
                <div className='text-muted-foreground text-[10.5px] leading-snug'>
                  {entry.description}
                </div>
              </div>
            </button>
          ) : (
            <Tooltip key={entry.label}>
              <TooltipTrigger asChild>
                <div className='flex w-full cursor-not-allowed items-start gap-2.5 rounded-lg border border-border px-2.5 py-2 opacity-45'>
                  <PaletteIcon entry={entry} />
                  <div className='min-w-0 flex-1'>
                    <div className='text-[12.5px] font-medium'>{entry.label}</div>
                    <div className='text-muted-foreground text-[10.5px] leading-snug'>
                      {entry.description}
                    </div>
                  </div>
                </div>
              </TooltipTrigger>
              <TooltipContent side='right'>{entry.reason}</TooltipContent>
            </Tooltip>
          )
        )}
      </div>
    </div>
  )
}

function PaletteIcon({ entry }: { entry: PaletteEntry }) {
  const Icon = entry.icon
  return (
    <div
      className={cn(
        'bg-accent text-primary grid size-7 shrink-0 place-items-center rounded-md',
        !entry.available && 'text-muted-foreground'
      )}
    >
      <Icon className='size-3.5' />
    </div>
  )
}
