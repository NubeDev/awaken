import { createFileRoute, redirect } from '@tanstack/react-router'

/**
 * The legacy History & SQL console is superseded by the Rules Studio query
 * workbench (a richer CodeMirror console with sortable results and a chart
 * toggle). Redirect to it so existing links keep working.
 */
export const Route = createFileRoute('/_authenticated/history/')({
  beforeLoad: () => {
    throw redirect({ to: '/rules', search: { tab: 'query' } })
  },
})
