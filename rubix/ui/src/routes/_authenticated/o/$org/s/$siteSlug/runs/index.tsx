import { createFileRoute } from '@tanstack/react-router'
import { Runs } from '@/features/runs'

export const Route = createFileRoute('/_authenticated/o/$org/s/$siteSlug/runs/')({
  component: Runs,
})
