import { createFileRoute } from '@tanstack/react-router'
import { Teams } from '@/features/admin/teams'

export const Route = createFileRoute('/_authenticated/o/$org/settings/teams')({
  component: Teams,
})
