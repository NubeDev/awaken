import { createFileRoute } from '@tanstack/react-router'
import { GatewayList } from '@/features/gateways'

export const Route = createFileRoute('/_authenticated/admin/gateways')({
  component: GatewayList,
})
