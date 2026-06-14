import { CircleAlert, Info, TriangleAlert } from 'lucide-react'
import type { SparkSeverity } from '@/api/types'
import { cn } from '@/lib/utils'

const MAP = {
  fault: { Icon: CircleAlert, color: 'text-sev-fault' },
  warning: { Icon: TriangleAlert, color: 'text-sev-warning' },
  info: { Icon: Info, color: 'text-sev-info' },
} as const

/** Coloured glyph for a spark severity, matching the badge palette. */
export function SeverityIcon({
  severity,
  className,
}: {
  severity: SparkSeverity
  className?: string
}) {
  const { Icon, color } = MAP[severity]
  return <Icon className={cn('size-4', color, className)} />
}
