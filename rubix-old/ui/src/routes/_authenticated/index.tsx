import { createFileRoute, redirect } from '@tanstack/react-router'
import * as api from '@/api/endpoints'

/**
 * The bare app root resolves to a concrete org route. We pick the first org the
 * principal can see (grouped from their visible sites) and send them to its
 * dashboards; with no sites yet, fall through to org settings to provision one.
 * Everything below `/o/$org` is shareable — the scope lives in the URL.
 */
export const Route = createFileRoute('/_authenticated/')({
  loader: async () => {
    const orgs = await api.orgs.list().catch(() => [])
    const first = orgs[0]?.org
    if (first) {
      throw redirect({ to: '/o/$org/dashboards', params: { org: first } })
    }
    throw redirect({ to: '/o/$org/settings/orgs', params: { org: 'new' } })
  },
})
