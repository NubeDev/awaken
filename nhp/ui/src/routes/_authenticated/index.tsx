import { createFileRoute, redirect } from '@tanstack/react-router'

/**
 * The app root lands on Dashboards. NHP has no org/site URL scope (unlike
 * rubix-old) — nav is flat. See WS-01.md.
 */
export const Route = createFileRoute('/_authenticated/')({
  loader: () => {
    throw redirect({ to: '/dashboards' })
  },
})
