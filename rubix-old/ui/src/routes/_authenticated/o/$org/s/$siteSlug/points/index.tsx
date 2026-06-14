import { z } from 'zod'
import { createFileRoute } from '@tanstack/react-router'
import { Points } from '@/features/points'

export const Route = createFileRoute('/_authenticated/o/$org/s/$siteSlug/points/')({
  validateSearch: z.object({ equip: z.string().optional().catch(undefined) }),
  component: Points,
})
