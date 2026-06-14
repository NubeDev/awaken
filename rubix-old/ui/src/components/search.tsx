import { Sparkles } from 'lucide-react'
import { cn } from '@/lib/utils'
import { useSearch } from '@/context/search-provider'
import { Button } from './ui/button'

/** Command-palette trigger styled as the "Ask awaken" entry point. */
export function Search({
  className = '',
  placeholder = 'Ask awaken or search…',
  ...props
}: React.ComponentProps<'button'> & { placeholder?: string }) {
  const { setOpen } = useSearch()
  return (
    <Button
      {...props}
      variant='outline'
      className={cn(
        'group relative h-9 w-full flex-1 justify-start rounded-lg bg-card text-sm font-normal text-muted-foreground shadow-sm hover:bg-accent sm:w-48 sm:pe-12 md:flex-none lg:w-64 xl:w-72',
        className
      )}
      aria-keyshortcuts='Meta+K Control+K'
      onClick={() => setOpen(true)}
    >
      <Sparkles
        aria-hidden='true'
        className='text-primary absolute inset-s-2.5 top-1/2 -translate-y-1/2'
        size={15}
      />
      <span className='ms-5 truncate text-[13px]'>{placeholder}</span>
      <kbd className='pointer-events-none absolute inset-e-[0.3rem] top-1/2 hidden h-5 -translate-y-1/2 items-center gap-1 rounded border bg-muted px-1.5 font-mono text-[10px] font-medium opacity-100 select-none group-hover:bg-accent sm:flex'>
        <span className='text-xs'>⌘</span>K
      </kbd>
    </Button>
  )
}
