import { FileCode2 } from 'lucide-react'
import type { RuleView, SparkSeverity } from '@/api/types'
import { cn } from '@/lib/utils'

/**
 * Derive a severity accent from what a rule's script emits, when derivable: the
 * highest-severity `finding("…")` literal in the source. Purely cosmetic (a left
 * accent bar) — the authoritative severity is the runtime verdict, not the
 * source — so an undetectable/dynamic severity simply shows no accent.
 */
function emittedSeverity(script: string): SparkSeverity | null {
  if (/finding\(\s*["']fault["']/.test(script)) return 'fault'
  if (/finding\(\s*["']warning["']/.test(script)) return 'warning'
  if (/finding\(\s*["']info["']/.test(script)) return 'info'
  return null
}

const ACCENT: Record<SparkSeverity, string> = {
  fault: 'border-l-sev-fault',
  warning: 'border-l-sev-warning',
  info: 'border-l-sev-info',
}

export function RuleList({
  rules,
  selectedName,
  onSelect,
}: {
  rules: RuleView[]
  selectedName: string | undefined
  onSelect: (name: string) => void
}) {
  if (rules.length === 0) {
    return (
      <p className='text-muted-foreground py-12 text-center text-sm'>
        No rules yet. Create one to get started.
      </p>
    )
  }
  return (
    <>
      {rules.map((rule) => {
        const sev = emittedSeverity(rule.script)
        const active = rule.name === selectedName
        const paramCount = Object.keys(rule.params.params).length
        return (
          <button
            key={rule.id}
            type='button'
            onClick={() => onSelect(rule.name)}
            aria-current={active}
            className={cn(
              'flex w-full items-start gap-2.5 rounded-md border border-l-2 border-transparent px-2.5 py-2 text-left',
              'hover:bg-muted/50 focus-visible:ring-ring focus-visible:ring-2 focus-visible:outline-none',
              sev ? ACCENT[sev] : 'border-l-border',
              active && 'bg-muted'
            )}
          >
            <FileCode2 className='text-muted-foreground mt-0.5 size-4 shrink-0' />
            <div className='min-w-0'>
              <div className='truncate font-mono text-[12.5px] font-medium'>
                {rule.name}
              </div>
              <div className='text-muted-foreground mt-0.5 text-[10.5px]'>
                {paramCount} param{paramCount === 1 ? '' : 's'}
              </div>
            </div>
          </button>
        )
      })}
    </>
  )
}
