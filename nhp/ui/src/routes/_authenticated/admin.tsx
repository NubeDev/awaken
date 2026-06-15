import { createFileRoute } from '@tanstack/react-router'
import { PlaceholderPage } from '@/features/placeholder/placeholder-page'

export const Route = createFileRoute('/_authenticated/admin')({
  component: () => (
    <PlaceholderPage title='Admin' owner='WS-04 / WS-05 (admin)' />
  ),
})
