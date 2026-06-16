import { createFileRoute } from '@tanstack/react-router'
import { DashboardPage } from '@/features/dashboards'

export const Route = createFileRoute('/_authenticated/dashboards')({
  component: DashboardPage,
})
