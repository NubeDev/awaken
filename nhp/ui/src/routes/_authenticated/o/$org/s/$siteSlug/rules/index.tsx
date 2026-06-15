import { createFileRoute } from '@tanstack/react-router'
import { Rules } from '@/features/rules'

type RulesSearch = { tab?: 'rules' | 'query' }

export const Route = createFileRoute('/_authenticated/o/$org/s/$siteSlug/rules/')({
  validateSearch: (search: Record<string, unknown>): RulesSearch => ({
    tab: search.tab === 'query' ? 'query' : undefined,
  }),
  component: Rules,
})
