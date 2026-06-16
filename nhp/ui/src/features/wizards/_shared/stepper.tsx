/**
 * The reusable wizard stepper shell (WS-06 task 1): a numbered step rail, the
 * active step's body, Back/Next nav with per-step validity gating, and a final
 * RUN phase that drives `runBatch` over the plan a wizard produced — showing
 * per-record results and a Retry that RESUMES (re-runs only the non-ok records),
 * never restarts (WIZARDS.md §Principles).
 *
 * A wizard supplies its `steps` (each renders its own body + reports `valid`) and
 * a `buildPlan()` called once the steps are complete; the stepper owns the
 * navigation, the preview, and the resumable write. One shell, every wizard.
 */
import { useState } from 'react'
import { Button } from '@/components/ui/button'
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card'
import { cn } from '@/lib/utils'
import {
  initialResults,
  runBatch,
  summarise,
  type BatchResults,
  type PlannedRecord,
} from './batch-write'
import { PreviewTable } from './preview-table'

export interface WizardStep {
  title: string
  /** The step body. */
  render: () => React.ReactNode
  /** Gate Next: false disables advancing past this step. */
  valid: boolean
}

export interface WizardShellProps {
  title: string
  description?: string
  steps: WizardStep[]
  /**
   * Build the records to create from the collected input. Called when the user
   * reaches the Preview phase (after the last input step). A wizard returns its
   * full ordered plan (parents before children, parentRefs for late-bound ids).
   */
  buildPlan: () => PlannedRecord[]
  /** Optional: fired after a fully-successful run (e.g. to reset or chain). */
  onComplete?: (results: BatchResults) => void
}

type Phase = 'input' | 'preview' | 'running' | 'done'

export function WizardShell({
  title,
  description,
  steps,
  buildPlan,
  onComplete,
}: WizardShellProps) {
  const [active, setActive] = useState(0)
  const [phase, setPhase] = useState<Phase>('input')
  const [plan, setPlan] = useState<PlannedRecord[]>([])
  const [results, setResults] = useState<BatchResults>({})

  const lastStep = active === steps.length - 1
  const current = steps[active]

  const goPreview = () => {
    const p = buildPlan()
    setPlan(p)
    setResults(initialResults(p))
    setPhase('preview')
  }

  const run = async () => {
    setPhase('running')
    const final = await runBatch(plan, results, setResults)
    setResults(final)
    setPhase('done')
    if (summarise(plan, final).error === 0) onComplete?.(final)
  }

  const summary = summarise(plan, results)

  // The step rail: input steps, then a Preview pseudo-step.
  const rail = [...steps.map((s) => s.title), 'Preview & write']
  const railActive = phase === 'input' ? active : steps.length

  return (
    <Card className='mx-auto max-w-4xl'>
      <CardHeader>
        <CardTitle>{title}</CardTitle>
        {description ? <CardDescription>{description}</CardDescription> : null}
        <ol className='mt-3 flex flex-wrap gap-2'>
          {rail.map((label, i) => (
            <li
              key={label}
              className={cn(
                'flex items-center gap-2 rounded-full border px-3 py-1 text-sm',
                i === railActive
                  ? 'border-primary text-foreground'
                  : 'text-muted-foreground'
              )}
            >
              <span className='font-mono text-xs'>{i + 1}</span>
              {label}
            </li>
          ))}
        </ol>
      </CardHeader>

      <CardContent className='space-y-4'>
        {phase === 'input' ? current.render() : null}

        {phase !== 'input' ? (
          <>
            {phase === 'done' ? (
              <div
                className={cn(
                  'rounded-md border p-3 text-sm',
                  summary.error === 0
                    ? 'border-emerald-500/40 text-emerald-600'
                    : 'border-amber-500/40 text-amber-600'
                )}
              >
                {summary.ok}/{summary.total} records written
                {summary.error > 0
                  ? ` — ${summary.error} failed; Retry resumes the rest.`
                  : ' — done.'}
              </div>
            ) : (
              <p className='text-muted-foreground text-sm'>
                {plan.length} records will be created with the tags shown. Nothing
                is written until you confirm.
              </p>
            )}
            <PreviewTable
              plan={plan}
              results={phase === 'preview' ? undefined : results}
            />
          </>
        ) : null}
      </CardContent>

      <div className='flex justify-between gap-2 border-t p-4'>
        <div>
          {phase === 'input' && active > 0 ? (
            <Button variant='ghost' onClick={() => setActive(active - 1)}>
              Back
            </Button>
          ) : null}
          {phase === 'preview' ? (
            <Button variant='ghost' onClick={() => setPhase('input')}>
              Back
            </Button>
          ) : null}
        </div>
        <div className='flex gap-2'>
          {phase === 'input' && !lastStep ? (
            <Button disabled={!current.valid} onClick={() => setActive(active + 1)}>
              Next
            </Button>
          ) : null}
          {phase === 'input' && lastStep ? (
            <Button disabled={!current.valid} onClick={goPreview}>
              Preview
            </Button>
          ) : null}
          {phase === 'preview' ? (
            <Button onClick={run} disabled={plan.length === 0}>
              Create {plan.length} records
            </Button>
          ) : null}
          {phase === 'running' ? <Button disabled>Writing…</Button> : null}
          {phase === 'done' && summary.error > 0 ? (
            <Button onClick={run}>Retry {summary.error} failed</Button>
          ) : null}
        </div>
      </div>
    </Card>
  )
}
