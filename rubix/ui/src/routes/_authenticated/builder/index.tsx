import { createFileRoute } from '@tanstack/react-router'
import { Builder } from '@/features/builder'

export const Route = createFileRoute('/_authenticated/builder/')({
  component: Builder,
})
