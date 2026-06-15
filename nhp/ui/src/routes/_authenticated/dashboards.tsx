import { createFileRoute } from '@tanstack/react-router'
import { PlaceholderPage } from '@/features/placeholder/placeholder-page'

export const Route = createFileRoute('/_authenticated/dashboards')({
  component: () => (
    <PlaceholderPage title='Dashboards' owner='WS-07 (dashboards)' />
  ),
})
