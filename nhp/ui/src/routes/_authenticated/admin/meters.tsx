import { createFileRoute } from '@tanstack/react-router'
import { MeterList } from '@/features/meters'

export const Route = createFileRoute('/_authenticated/admin/meters')({
  component: MeterList,
})
