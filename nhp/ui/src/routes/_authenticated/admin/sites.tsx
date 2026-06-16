import { createFileRoute } from '@tanstack/react-router'
import { SiteList } from '@/features/sites'

export const Route = createFileRoute('/_authenticated/admin/sites')({
  component: SiteList,
})
