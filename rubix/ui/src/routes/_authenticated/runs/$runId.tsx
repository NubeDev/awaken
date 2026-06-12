import { createFileRoute } from '@tanstack/react-router'
import { RunDetail } from '@/features/runs/detail'

export const Route = createFileRoute('/_authenticated/runs/$runId')({
  component: RunDetailRoute,
})

function RunDetailRoute() {
  const { runId } = Route.useParams()
  return <RunDetail runId={runId} />
}
