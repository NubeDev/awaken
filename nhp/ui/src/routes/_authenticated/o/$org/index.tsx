import { createFileRoute, redirect } from '@tanstack/react-router'

/** `/o/$org` lands on the org's dashboards. */
export const Route = createFileRoute('/_authenticated/o/$org/')({
  loader: ({ params }) => {
    throw redirect({ to: '/o/$org/dashboards', params: { org: params.org } })
  },
})
