/**
 * Per-resource History tab (docs/design/audit-and-undo.md "UI"): the change
 * timeline for one resource (`GET /api/v1/audit/{kind}/{id}`), powering a
 * "History" tab on a dashboard, datasource, rule, … . Reuses the shared
 * `ChangeTimeline`, so the timeline + before→after diff are consistent with the
 * admin Audit screen.
 *
 * The timeline route is admin-gated server-side; a non-admin caller receives a
 * 403 surfaced as the error state below rather than leaking another view.
 */
import { useResourceHistory } from '@/api/hooks'
import type { Uuid } from '@/api/types'
import { Card } from '@/components/ui/card'
import { ChangeTimeline } from './change-timeline'

export function HistoryTab({ kind, id }: { kind: string; id: Uuid }) {
  const { data, isLoading, isError, error } = useResourceHistory(kind, id)

  if (isLoading) {
    return (
      <Card className='grid h-40 place-items-center'>
        <p className='text-sm text-muted-foreground'>Loading history…</p>
      </Card>
    )
  }

  if (isError) {
    return (
      <Card className='grid h-40 place-items-center'>
        <p className='text-sm text-sev-fault'>
          {(error as Error).message}
        </p>
      </Card>
    )
  }

  return (
    <ChangeTimeline
      changes={data ?? []}
      emptyLabel='No changes recorded for this resource yet.'
    />
  )
}
