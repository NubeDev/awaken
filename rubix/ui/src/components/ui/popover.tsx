// A minimal Popover built on Radix DropdownMenu (already vendored) so we get a
// portalled, outside-click-dismissing floating panel without adding
// `@radix-ui/react-popover`. The board time-range picker is the only consumer:
// a trigger that shows the current range, opening a panel of quick ranges +
// custom inputs. `DropdownMenu`'s focus/roving behaviour is fine here because the
// panel holds plain buttons and inputs (we stop propagation on the inputs so
// typing isn't swallowed by the menu's typeahead).

import * as React from 'react'
import * as DropdownMenuPrimitive from '@radix-ui/react-dropdown-menu'
import { cn } from '@/lib/cn'

function Popover({ ...props }: React.ComponentProps<typeof DropdownMenuPrimitive.Root>) {
  return <DropdownMenuPrimitive.Root data-slot="popover" {...props} />
}

function PopoverTrigger({ ...props }: React.ComponentProps<typeof DropdownMenuPrimitive.Trigger>) {
  return <DropdownMenuPrimitive.Trigger data-slot="popover-trigger" {...props} />
}

function PopoverContent({
  className,
  align = 'start',
  sideOffset = 6,
  ...props
}: React.ComponentProps<typeof DropdownMenuPrimitive.Content>) {
  return (
    <DropdownMenuPrimitive.Portal>
      <DropdownMenuPrimitive.Content
        data-slot="popover-content"
        align={align}
        sideOffset={sideOffset}
        className={cn(
          'z-50 rounded-xl border border-border bg-popover p-3 text-popover-foreground shadow-lg outline-hidden',
          'data-[state=open]:animate-in data-[state=open]:fade-in-0 data-[state=open]:zoom-in-95',
          'data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=closed]:zoom-out-95',
          className,
        )}
        {...props}
      />
    </DropdownMenuPrimitive.Portal>
  )
}

export { Popover, PopoverTrigger, PopoverContent }
