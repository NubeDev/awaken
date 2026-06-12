import { createFileRoute } from '@tanstack/react-router'
import { Flows } from '@/features/flows'

export const Route = createFileRoute('/_authenticated/flows/')({
  component: Flows,
})
