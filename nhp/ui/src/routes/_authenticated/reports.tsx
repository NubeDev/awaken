import { createFileRoute } from '@tanstack/react-router'
import { ReportsPage } from '@/features/reporting'

export const Route = createFileRoute('/_authenticated/reports')({
  component: ReportsPage,
})
