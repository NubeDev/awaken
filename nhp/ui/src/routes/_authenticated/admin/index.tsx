import { createFileRoute, redirect } from '@tanstack/react-router'

/** /admin lands on the tenants section — the top of the portfolio hierarchy. */
export const Route = createFileRoute('/_authenticated/admin/')({
  loader: () => {
    throw redirect({ to: '/admin/tenants' })
  },
})
