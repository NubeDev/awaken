import { createFileRoute } from '@tanstack/react-router'
import { Members } from '@/features/admin/members'

export const Route = createFileRoute('/_authenticated/o/$org/settings/members')({
  component: Members,
})
