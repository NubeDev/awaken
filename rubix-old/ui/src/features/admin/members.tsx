import { useState } from 'react'
import { Pencil, Plus, Trash2, UserCog } from 'lucide-react'
import {
  useCreateUser,
  useDeleteUser,
  usePatchUser,
  useUsers,
} from '@/api/hooks'
import type { AdminLevel, User } from '@/api/types'
import { useScope } from '@/context/scope-provider'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card } from '@/components/ui/card'
import {
  Dialog,
  DialogContent,
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
import { ConfirmDialog } from '@/components/confirm-dialog'
import { Main } from '@/components/layout/main'
import { PageHeader } from '@/components/layout/page-header'
import { AdminGuard } from './admin-guard'

const ADMIN_LEVELS: { value: AdminLevel; label: string }[] = [
  { value: 'none', label: 'Member' },
  { value: 'org_admin', label: 'Org admin' },
  { value: 'super_admin', label: 'Super admin' },
]

const levelLabel = (l: AdminLevel) =>
  ADMIN_LEVELS.find((x) => x.value === l)?.label ?? l

/**
 * Members: the org's users and their admin tier. Create binds a user to a
 * verified token `subject`; edit changes the email / display name / admin level.
 * Gated on `whoami.can_admin`. See docs/design/authz-rbac.md increment E.
 */
export function Members() {
  return (
    <AdminGuard title='Members' sub='Org users and their admin level'>
      <MembersBody />
    </AdminGuard>
  )
}

function MembersBody() {
  const { org } = useScope()
  const { data: users = [] } = useUsers(org)
  const [createOpen, setCreateOpen] = useState(false)

  return (
    <>
      <PageHeader title='Members' sub='Org users and their admin level' />
      <Main fluid fixed className='flex min-h-0 flex-col'>
        <div className='mb-3 flex items-center justify-between'>
          <p className='text-sm text-muted-foreground'>
            {users.length} {users.length === 1 ? 'member' : 'members'}
          </p>
          <Button size='sm' onClick={() => setCreateOpen(true)} disabled={!org}>
            <Plus className='size-4' /> Add member
          </Button>
        </div>

        <div className='scroll min-h-0 flex-1 space-y-1.5 overflow-y-auto pe-1'>
          {users.length === 0 ? (
            <Card className='grid h-40 place-items-center'>
              <p className='text-sm text-muted-foreground'>No members yet.</p>
            </Card>
          ) : (
            users.map((u) => <MemberRow key={u.id} org={org as string} user={u} />)
          )}
        </div>
      </Main>

      {org ? (
        <UserFormDialog
          mode='create'
          org={org}
          open={createOpen}
          onOpenChange={setCreateOpen}
        />
      ) : null}
    </>
  )
}

function MemberRow({ org, user }: { org: string; user: User }) {
  const del = useDeleteUser(org)
  const [editOpen, setEditOpen] = useState(false)
  const [confirmOpen, setConfirmOpen] = useState(false)

  return (
    <div className='flex items-center justify-between rounded-md bg-muted/40 px-3 py-2'>
      <div className='min-w-0'>
        <div className='flex items-center gap-2'>
          <UserCog className='size-4 shrink-0 text-primary' />
          <span className='truncate text-sm'>{user.display_name}</span>
          <code className='text-[11px] text-muted-foreground'>{user.email}</code>
          {user.admin_level !== 'none' ? (
            <Badge variant='muted' className='h-4 px-1.5 text-[10px]'>
              {levelLabel(user.admin_level)}
            </Badge>
          ) : null}
        </div>
        <code className='text-[10px] text-muted-foreground'>
          subject: {user.subject}
        </code>
      </div>
      <div className='flex shrink-0 items-center gap-1'>
        <Button
          size='icon'
          variant='ghost'
          className='size-7'
          onClick={() => setEditOpen(true)}
        >
          <Pencil className='size-3.5' />
        </Button>
        <Button
          size='icon'
          variant='ghost'
          className='size-7 text-sev-fault'
          onClick={() => setConfirmOpen(true)}
        >
          <Trash2 className='size-3.5' />
        </Button>
      </div>

      <UserFormDialog
        mode='edit'
        org={org}
        user={user}
        open={editOpen}
        onOpenChange={setEditOpen}
      />
      <ConfirmDialog
        open={confirmOpen}
        onOpenChange={setConfirmOpen}
        destructive
        title={`Remove ${user.display_name}?`}
        desc={
          <>
            This removes the user and all their team memberships. Direct grants
            to this user are also dropped.
          </>
        }
        confirmText='Remove member'
        isLoading={del.isPending}
        handleConfirm={() =>
          del.mutate(user.id, { onSuccess: () => setConfirmOpen(false) })
        }
      />
    </div>
  )
}

type UserFormProps = {
  open: boolean
  onOpenChange: (open: boolean) => void
  org: string
} & ({ mode: 'create'; user?: undefined } | { mode: 'edit'; user: User })

function UserFormDialog(props: UserFormProps) {
  const { open, onOpenChange } = props
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className='sm:max-w-md'>
        {open ? <UserFormBody {...props} /> : null}
      </DialogContent>
    </Dialog>
  )
}

function UserFormBody(props: UserFormProps) {
  const { onOpenChange, mode, org } = props
  const create = useCreateUser(org)
  const patch = usePatchUser(org)

  const [subject, setSubject] = useState(
    mode === 'edit' ? props.user.subject : ''
  )
  const [email, setEmail] = useState(mode === 'edit' ? props.user.email : '')
  const [displayName, setDisplayName] = useState(
    mode === 'edit' ? props.user.display_name : ''
  )
  const [adminLevel, setAdminLevel] = useState<AdminLevel>(
    mode === 'edit' ? props.user.admin_level : 'none'
  )
  const [error, setError] = useState<string | null>(null)
  const pending = create.isPending || patch.isPending

  const submit = () => {
    if (!email.trim() || !displayName.trim()) {
      setError('Email and display name are required.')
      return
    }
    const onError = (e: unknown) => setError((e as Error).message)
    const onSuccess = () => onOpenChange(false)
    if (mode === 'create') {
      if (!subject.trim()) {
        setError('Subject (token id / OIDC sub) is required.')
        return
      }
      create.mutate(
        {
          subject: subject.trim(),
          email: email.trim(),
          display_name: displayName.trim(),
          admin_level: adminLevel,
        },
        { onSuccess, onError }
      )
    } else {
      patch.mutate(
        {
          id: props.user.id,
          body: {
            email: email.trim(),
            display_name: displayName.trim(),
            admin_level: adminLevel,
          },
        },
        { onSuccess, onError }
      )
    }
  }

  return (
    <>
      <DialogHeader>
        <DialogTitle>{mode === 'create' ? 'Add member' : 'Edit member'}</DialogTitle>
      </DialogHeader>
      <div className='space-y-3 py-1'>
        <div className='space-y-1.5'>
          <Label className='text-[12px]'>Subject</Label>
          <Input
            value={subject}
            onChange={(e) => setSubject(e.target.value)}
            placeholder='OIDC sub or PAT id'
            disabled={mode === 'edit'}
          />
        </div>
        <div className='space-y-1.5'>
          <Label className='text-[12px]'>Email</Label>
          <Input
            value={email}
            onChange={(e) => setEmail(e.target.value)}
            placeholder='name@org.com'
          />
        </div>
        <div className='space-y-1.5'>
          <Label className='text-[12px]'>Display name</Label>
          <Input
            value={displayName}
            onChange={(e) => setDisplayName(e.target.value)}
            placeholder='Jane Operator'
          />
        </div>
        <div className='space-y-1.5'>
          <Label className='text-[12px]'>Admin level</Label>
          <Select
            value={adminLevel}
            onValueChange={(v) => setAdminLevel(v as AdminLevel)}
          >
            <SelectTrigger>
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {ADMIN_LEVELS.map((l) => (
                <SelectItem key={l.value} value={l.value}>
                  {l.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
        {error ? <p className='text-[12px] text-sev-fault'>{error}</p> : null}
      </div>
      <DialogFooter>
        <Button variant='ghost' onClick={() => onOpenChange(false)}>
          Cancel
        </Button>
        <Button onClick={submit} disabled={pending}>
          {pending ? 'Saving…' : mode === 'create' ? 'Create' : 'Save changes'}
        </Button>
      </DialogFooter>
    </>
  )
}
