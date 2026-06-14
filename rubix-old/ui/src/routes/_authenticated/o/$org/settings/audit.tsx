import { createFileRoute } from '@tanstack/react-router'
import { AuditScreen } from '@/features/audit/audit-screen'

export const Route = createFileRoute('/_authenticated/o/$org/settings/audit')({
  component: AuditScreen,
})
