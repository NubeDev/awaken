import { createFileRoute } from '@tanstack/react-router'
import { MeterTypeList } from '@/features/meter-types'

export const Route = createFileRoute('/_authenticated/admin/meter-types')({
  component: MeterTypeList,
})
