/**
 * Searchable IANA timezone picker. The list comes from the runtime itself
 * (`Intl.supportedValuesOf('timeZone')`) so it needs no dependency and stays
 * current with the platform. Used by the site form, where the zone is usually
 * auto-derived from the picked coordinates but stays freely editable.
 */
import { useMemo, useState } from 'react'
import { Check, ChevronsUpDown } from 'lucide-react'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
} from '@/components/ui/command'
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from '@/components/ui/popover'

function allZones(): string[] {
  const intl = Intl as typeof Intl & {
    supportedValuesOf?: (key: string) => string[]
  }
  try {
    return intl.supportedValuesOf?.('timeZone') ?? []
  } catch {
    return []
  }
}

type TimezoneSelectProps = {
  id?: string
  value: string
  onChange: (tz: string) => void
}

export function TimezoneSelect({ id, value, onChange }: TimezoneSelectProps) {
  const [open, setOpen] = useState(false)
  const zones = useMemo(() => allZones(), [])

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <Button
          id={id}
          type='button'
          variant='outline'
          role='combobox'
          aria-expanded={open}
          className='justify-between font-normal'
        >
          <span className={cn(!value && 'text-muted-foreground')}>
            {value || 'Select timezone'}
          </span>
          <ChevronsUpDown className='ms-1 size-4 opacity-50' />
        </Button>
      </PopoverTrigger>
      <PopoverContent className='w-[--radix-popover-trigger-width] p-0' align='start'>
        <Command>
          <CommandInput placeholder='Search timezones…' />
          <CommandList>
            <CommandEmpty>No timezone found.</CommandEmpty>
            <CommandGroup>
              {zones.map((z) => (
                <CommandItem
                  key={z}
                  value={z}
                  onSelect={() => {
                    onChange(z)
                    setOpen(false)
                  }}
                >
                  <Check
                    className={cn('size-4', value === z ? 'opacity-100' : 'opacity-0')}
                  />
                  {z}
                </CommandItem>
              ))}
            </CommandGroup>
          </CommandList>
        </Command>
      </PopoverContent>
    </Popover>
  )
}
