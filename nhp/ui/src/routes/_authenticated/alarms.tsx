import { createFileRoute } from '@tanstack/react-router'
import { AlarmsConsole } from '@/features/reporting'

export const Route = createFileRoute('/_authenticated/alarms')({
  component: AlarmsConsole,
})
