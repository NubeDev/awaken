import { createFileRoute } from '@tanstack/react-router'
import { Sparks } from '@/features/sparks'

export const Route = createFileRoute('/_authenticated/o/$org/s/$siteSlug/sparks/')({
  component: Sparks,
})
