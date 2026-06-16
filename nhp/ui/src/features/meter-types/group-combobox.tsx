/**
 * Group picker for a register's `chart_group` (DOMAIN-MODEL §register). Replaces
 * the old free-text box: autocompletes from groups already used on sibling
 * registers and lets you create a new one by typing. Keeps grouping consistent
 * (no "Voltage" vs "voltage" drift) while staying open-ended.
 */
import { useState } from 'react'
import { Check, ChevronsUpDown, Plus, X } from 'lucide-react'
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

type GroupComboboxProps = {
  value: string
  /** Distinct groups already used across the meter-type's registers. */
  options: string[]
  onChange: (group: string) => void
  className?: string
}

export function GroupCombobox({
  value,
  options,
  onChange,
  className,
}: GroupComboboxProps) {
  const [open, setOpen] = useState(false)
  const [query, setQuery] = useState('')

  const trimmed = query.trim()
  const canCreate =
    trimmed.length > 0 &&
    !options.some((o) => o.toLowerCase() === trimmed.toLowerCase())

  const pick = (group: string) => {
    onChange(group)
    setQuery('')
    setOpen(false)
  }

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <Button
          type='button'
          variant='outline'
          role='combobox'
          aria-expanded={open}
          className={cn('h-8 justify-between font-normal', className)}
        >
          <span className={cn(!value && 'text-muted-foreground')}>
            {value || 'No group'}
          </span>
          <ChevronsUpDown className='ms-1 size-3 opacity-50' />
        </Button>
      </PopoverTrigger>
      <PopoverContent className='w-56 p-0' align='start'>
        <Command>
          <CommandInput
            placeholder='Find or create group…'
            value={query}
            onValueChange={setQuery}
          />
          <CommandList>
            <CommandEmpty>No matching group.</CommandEmpty>
            <CommandGroup>
              {value ? (
                <CommandItem value='__none__' onSelect={() => pick('')}>
                  <X className='size-4 opacity-60' />
                  Clear group
                </CommandItem>
              ) : null}
              {options.map((o) => (
                <CommandItem key={o} value={o} onSelect={() => pick(o)}>
                  <Check
                    className={cn(
                      'size-4',
                      value === o ? 'opacity-100' : 'opacity-0'
                    )}
                  />
                  {o}
                </CommandItem>
              ))}
              {canCreate ? (
                <CommandItem
                  value={`__create__${trimmed}`}
                  onSelect={() => pick(trimmed)}
                >
                  <Plus className='size-4' />
                  Create “{trimmed}”
                </CommandItem>
              ) : null}
            </CommandGroup>
          </CommandList>
        </Command>
      </PopoverContent>
    </Popover>
  )
}
