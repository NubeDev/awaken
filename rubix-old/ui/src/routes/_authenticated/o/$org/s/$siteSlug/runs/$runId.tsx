import { createFileRoute } from '@tanstack/react-router'
import { RunDetail } from '@/features/runs/detail'

export const Route = createFileRoute('/_authenticated/o/$org/s/$siteSlug/runs/$runId')({
  component: RunDetailRoute,
})

// eslint-disable-next-line react-refresh/only-export-components
function RunDetailRoute() {
  const { runId } = Route.useParams()
  return <RunDetail runId={runId} />
}
