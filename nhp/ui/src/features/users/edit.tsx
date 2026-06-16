/**
 * Create a principal (ADMIN.md §6): subject (namespace-local), kind (user vs
 * extension service account), NHP role, and an optional secret (omit to have the
 * server mint and return one once — surfaced in the success toast by hooks.ts).
 *
 * Role change and delete are inline on the list (role-select.tsx); this form is
 * create-only. POSTs `/principals` with the admin credential (api/admin.ts).
 */
import { useState } from 'react'
import type { CreatePrincipalBody, PrincipalKind, PrincipalRole } from '@/api/admin'
import { ROLE, toOptions } from '@/enums/options'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { useCreatePrincipal } from './hooks'

const KINDS: { value: PrincipalKind; label: string }[] = [
  { value: 'user', label: 'User' },
  { value: 'extension', label: 'Service account' },
]

export function UserForm({
  open,
  onOpenChange,
}: {
  open: boolean
  onOpenChange: (open: boolean) => void
}) {
  const [subject, setSubject] = useState('')
  const [kind, setKind] = useState<PrincipalKind>('user')
  const [role, setRole] = useState<PrincipalRole>('viewer')
  const [secret, setSecret] = useState('')

  const create = useCreatePrincipal()
  const valid = subject.trim() !== ''

  const save = () => {
    const body: CreatePrincipalBody = {
      subject: subject.trim(),
      kind,
      role,
      // Empty secret => omit, so the server mints one and returns it once.
      ...(secret.trim() ? { secret: secret.trim() } : {}),
    }
    create.mutate(body, { onSuccess: () => onOpenChange(false) })
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className='max-w-md'>
        <DialogHeader>
          <DialogTitle>New principal</DialogTitle>
          <DialogDescription>
            A user or service account in this tenant. The subject is local to the
            namespace (the server prefixes it). Leave the secret blank to have one
            generated — it is shown only once.
          </DialogDescription>
        </DialogHeader>

        <div className='grid gap-4'>
          <div className='grid gap-1'>
            <Label htmlFor='u-subject'>Subject</Label>
            <Input
              id='u-subject'
              value={subject}
              onChange={(e) => setSubject(e.target.value)}
              placeholder='alice'
            />
          </div>
          <div className='grid gap-4 sm:grid-cols-2'>
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
                  {toOptions(ROLE).map((o) => (
                    <SelectItem key={o.value} value={o.value}>
                      {o.label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
          </div>
          <div className='grid gap-1'>
            <Label htmlFor='u-secret'>Secret (optional)</Label>
            <Input
              id='u-secret'
              value={secret}
              onChange={(e) => setSecret(e.target.value)}
              placeholder='leave blank to auto-generate'
            />
          </div>
        </div>

        <DialogFooter>
          <Button variant='ghost' onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button onClick={save} disabled={!valid || create.isPending}>
            {create.isPending ? 'Creating…' : 'Create'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
