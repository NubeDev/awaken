import { createFileRoute, redirect } from '@tanstack/react-router'

/** Legacy History & SQL is folded into the Rules Studio query workbench. */
export const Route = createFileRoute('/_authenticated/o/$org/s/$siteSlug/history/')({
  beforeLoad: ({ params }) => {
    throw redirect({
      to: '/o/$org/s/$siteSlug/rules',
      params: { org: params.org, siteSlug: params.siteSlug },
      search: { tab: 'query' },
    })
  },
})
