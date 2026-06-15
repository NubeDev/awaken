import { createFileRoute } from '@tanstack/react-router'
import { PlaceholderPage } from '@/features/placeholder/placeholder-page'

export const Route = createFileRoute('/_authenticated/wizards')({
  component: () => (
    <PlaceholderPage title='Wizards' owner='WS-06 (onboarding wizards)' />
  ),
})
