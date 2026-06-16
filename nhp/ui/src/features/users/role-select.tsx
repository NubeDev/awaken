/**
 * Inline NHP-role selector for a principal (viewer / operator / admin — ADMIN.md
 * §6). Changing it PATCHes `/principals/:subject { role }`. The rubix gate refuses
 * to demote the last admin (409); that error surfaces via the toast.
 */
import type { PrincipalRole } from '@/api/admin'
import { ROLE, toOptions } from '@/enums/options'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { useSetRole } from './hooks'

export function RoleSelect({
  subject,
  role,
}: {
  subject: string
  role: PrincipalRole
}) {
  const setRole = useSetRole()
  return (
    <Select
      value={role}
      onValueChange={(next) =>
        setRole.mutate({ subject, role: next as PrincipalRole })
      }
      disabled={setRole.isPending}
    >
      <SelectTrigger className='h-8 w-32'>
        <SelectValue />
      </SelectTrigger>
      <SelectContent>
        {toOptions(ROLE).map((o) => (
          <SelectItem key={o.value} value={o.value}>
            {o.label}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  )
}
