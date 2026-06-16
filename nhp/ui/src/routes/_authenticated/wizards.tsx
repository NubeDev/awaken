import { createFileRoute } from '@tanstack/react-router'
import { WizardsPage } from '@/features/wizards'

export const Route = createFileRoute('/_authenticated/wizards')({
  component: WizardsPage,
})
