import { createFileRoute } from '@tanstack/react-router'
import { UserList } from '@/features/users'

export const Route = createFileRoute('/_authenticated/admin/users')({
  component: UserList,
})
