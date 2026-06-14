import { createFileRoute } from '@tanstack/react-router'
import { NavigationBuilder } from '@/features/nav/navigation-builder'

export const Route = createFileRoute(
  '/_authenticated/o/$org/settings/navigation'
)({
  component: NavigationBuilder,
})
