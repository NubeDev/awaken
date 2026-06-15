// shadcn/ui Skeleton. Used by the sidebar's loading placeholders.

import { cn } from '@/lib/cn'

function Skeleton({ className, ...props }: React.ComponentProps<'div'>) {
  return <div data-slot="skeleton" className={cn('animate-pulse rounded-md bg-accent', className)} {...props} />
}

export { Skeleton }
