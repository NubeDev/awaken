import { createFileRoute } from '@tanstack/react-router'
import { DataConsole } from '@/features/data-console'

export const Route = createFileRoute('/_authenticated/admin/data-console')({
  component: DataConsole,
})
