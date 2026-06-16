import { createFileRoute, redirect } from '@tanstack/react-router'

/** /admin lands on the meter-types section (WS-04 owns admin's first surface). */
export const Route = createFileRoute('/_authenticated/admin/')({
  loader: () => {
    throw redirect({ to: '/admin/meter-types' })
  },
})
