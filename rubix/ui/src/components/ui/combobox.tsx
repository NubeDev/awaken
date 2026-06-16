// shadcn/ui Combobox (new-york): a Popover + Command pairing — a typeable,
// searchable, keyboard-navigable single-select. Built on the real
// `@radix-ui/react-popover` (the vendored `popover.tsx` is a dropdown-menu shim
// kept for the board time-range picker; a combobox needs Popover's anchored,
// non-menu focus model so the Command input owns the keyboard).
//
// Suggests without constraining: when `allowCustom` is set, a value the option
// list does not contain is still committable (type it, press Enter / pick the
// "Use …" row) — so a discovered-but-incomplete list (a new series, an unscanned
// field) never blocks a valid binding.

import * as React from 'react'
import * as PopoverPrimitive from '@radix-ui/react-popover'
import { Check, ChevronsUpDown } from 'lucide-react'

import { cn } from '@/lib/cn'
import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
} from './command'

export interface ComboboxProps {
  /** The current value (free text — not necessarily one of `options`). */
  value: string
  /** Commit a new value (a picked option or, with `allowCustom`, typed text). */
  onChange: (value: string) => void
  /** The discovered suggestions to offer. */
  options: string[]
  /** Trigger placeholder when `value` is empty. */
  placeholder?: string
  /** Search-input placeholder. */
  searchPlaceholder?: string
  /** Allow committing a typed value not present in `options`. Default true. */
  allowCustom?: boolean
  /** Shown above the list when `options` was capped server-side. */
  truncatedNote?: string
  /** Extra classes for the trigger (width/typography). */
  className?: string
  /** Accessible label for the trigger. */
  'aria-label'?: string
  /** Trigger content when there are no options and no value (e.g. "loading…"). */
  emptyLabel?: string
}

export function Combobox({
  value,
  onChange,
  options,
  placeholder,
  searchPlaceholder = 'Search…',
  allowCustom = true,
  truncatedNote,
  className,
  emptyLabel,
  'aria-label': ariaLabel,
}: ComboboxProps) {
  const [open, setOpen] = React.useState(false)
  const [search, setSearch] = React.useState('')

  const commit = (next: string) => {
    onChange(next)
    setOpen(false)
    setSearch('')
  }

  // Offer the typed text as a custom commit when it is non-empty and not already
  // an exact option — the escape hatch for an unlisted series/field.
  const trimmed = search.trim()
  const showCustom =
    allowCustom && trimmed.length > 0 && !options.some((o) => o === trimmed)

  return (
    <PopoverPrimitive.Root open={open} onOpenChange={setOpen}>
      <PopoverPrimitive.Trigger asChild>
        <button
          type="button"
          role="combobox"
          aria-expanded={open}
          aria-label={ariaLabel}
          className={cn(
            'mono flex h-8 items-center justify-between gap-1 rounded-md border border-input bg-card/40 px-2.5 text-[12px] shadow-sm transition-colors',
            'focus-visible:border-ring/60 focus-visible:outline-none focus-visible:ring-4 focus-visible:ring-ring/10',
            'data-[state=open]:border-ring/60',
            className,
          )}
        >
          <span className={cn('truncate', !value && 'text-muted-foreground')}>
            {value || placeholder || emptyLabel || 'Select…'}
          </span>
          <ChevronsUpDown className="size-3.5 shrink-0 opacity-50" />
        </button>
      </PopoverPrimitive.Trigger>
      <PopoverPrimitive.Portal>
        <PopoverPrimitive.Content
          align="start"
          sideOffset={6}
          className={cn(
            'z-50 w-[var(--radix-popover-trigger-width)] min-w-[220px] overflow-hidden rounded-xl border border-border bg-popover p-0 text-popover-foreground shadow-lg outline-hidden',
            'data-[state=open]:animate-in data-[state=open]:fade-in-0 data-[state=open]:zoom-in-95',
            'data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=closed]:zoom-out-95',
          )}
        >
          <Command
            // Custom rows carry their own filtering; keep cmdk's default scoring
            // for option rows.
            filter={(itemValue, query) =>
              itemValue.toLowerCase().includes(query.toLowerCase()) ? 1 : 0
            }
          >
            <CommandInput
              value={search}
              onValueChange={setSearch}
              placeholder={searchPlaceholder}
            />
            {truncatedNote && (
              <p className="border-b px-3 py-1.5 text-[10.5px] text-muted-foreground">
                {truncatedNote}
              </p>
            )}
            <CommandList>
              {!showCustom && <CommandEmpty>No matches.</CommandEmpty>}
              {showCustom && (
                <CommandGroup>
                  <CommandItem value={trimmed} onSelect={() => commit(trimmed)}>
                    Use “{trimmed}”
                  </CommandItem>
                </CommandGroup>
              )}
              {options.length > 0 && (
                <CommandGroup>
                  {options.map((opt) => (
                    <CommandItem key={opt} value={opt} onSelect={() => commit(opt)}>
                      <Check
                        className={cn('size-3.5', opt === value ? 'opacity-100' : 'opacity-0')}
                      />
                      <span className="mono truncate text-[12px]">{opt}</span>
                    </CommandItem>
                  ))}
                </CommandGroup>
              )}
            </CommandList>
          </Command>
        </PopoverPrimitive.Content>
      </PopoverPrimitive.Portal>
    </PopoverPrimitive.Root>
  )
}
