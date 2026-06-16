/**
 * New-user wizard (WS-06 task 4). Users are PRINCIPALS on the rubix admin surface
 * (/principals), NOT records — so this wizard reuses WS-05's `useCreatePrincipal`
 * (admin-authed) rather than the generic records batch writer. A principal is
 * subject + kind + role (+ optional secret; blank ⇒ server mints one, surfaced
 * once). "Teams" have no backend object in the POC (WS-05 §reachability) — role is
 * the membership model. This is a thin guided form over the existing create path.
 */
import { useState } from 'react'
import type { PrincipalKind, PrincipalRole } from '@/api/admin'
import { ROLE } from '@/enums/options'
import { Button } from '@/components/ui/button'
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { useCreatePrincipal } from '@/features/users/hooks'

const KINDS: { value: PrincipalKind; label: string }[] = [
  { value: 'user', label: 'User' },
  { value: 'extension', label: 'Service account' },
]

export function UserWizard() {
  const [subject, setSubject] = useState('')
  const [kind, setKind] = useState<PrincipalKind>('user')
  const [role, setRole] = useState<PrincipalRole>('viewer')
  const [secret, setSecret] = useState('')
  const create = useCreatePrincipal()

  const valid = subject.trim() !== ''

  const submit = () =>
    create.mutate({
      subject: subject.trim(),
      kind,
      role,
      secret: secret.trim() || undefined,
    })

  return (
    <Card className='mx-auto max-w-2xl'>
      <CardHeader>
        <CardTitle>New user</CardTitle>
        <CardDescription>
          Add a principal (user or service account) with a role. Subjects are
          namespace-local; a blank secret is server-minted and shown once.
        </CardDescription>
      </CardHeader>
      <CardContent className='grid gap-4'>
        <div className='grid gap-4 sm:grid-cols-2'>
          <div className='grid gap-1'>
            <Label htmlFor='u-subject'>Subject</Label>
            <Input
              id='u-subject'
              value={subject}
              onChange={(e) => setSubject(e.target.value)}
              placeholder='alice'
            />
          </div>
          <div className='grid gap-1'>
            <Label htmlFor='u-kind'>Kind</Label>
            <Select
              value={kind}
              onValueChange={(v) => setKind(v as PrincipalKind)}
            >
              <SelectTrigger id='u-kind'>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {KINDS.map((k) => (
                  <SelectItem key={k.value} value={k.value}>
                    {k.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
        </div>
        <div className='grid gap-4 sm:grid-cols-2'>
          <div className='grid gap-1'>
            <Label htmlFor='u-role'>Role</Label>
            <Select
              value={role}
              onValueChange={(v) => setRole(v as PrincipalRole)}
            >
              <SelectTrigger id='u-role'>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {ROLE.map((r) => (
                  <SelectItem key={r} value={r}>
                    {r}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
          <div className='grid gap-1'>
            <Label htmlFor='u-secret'>Secret (optional)</Label>
            <Input
              id='u-secret'
              value={secret}
              onChange={(e) => setSecret(e.target.value)}
              placeholder='blank → server mints one'
            />
          </div>
        </div>
        <div className='flex justify-end'>
          <Button onClick={submit} disabled={!valid || create.isPending}>
            {create.isPending ? 'Creating…' : 'Create principal'}
          </Button>
        </div>
      </CardContent>
    </Card>
  )
}
