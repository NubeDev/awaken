// The shadcn class-merge helper: compose conditional class lists (clsx) and let
// later Tailwind utilities win over earlier ones (tailwind-merge). Every vendored
// ui/ primitive uses this so a caller's `className` overrides the default.

import { clsx, type ClassValue } from 'clsx'
import { twMerge } from 'tailwind-merge'

export function cn(...inputs: ClassValue[]): string {
  return twMerge(clsx(inputs))
}
