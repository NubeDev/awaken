import { useState } from 'react'
import { Plus, Trash2, Users, X } from 'lucide-react'
import {
  useAddTeamMember,
  useCreateTeam,
  useDeleteTeam,
  useRemoveTeamMember,
  useTeamMembers,
  useTeams,
  useUsers,
} from '@/api/hooks'
import type { Team, User } from '@/api/types'
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

/**
 * Teams: named groups within an org. Create a team, then add/remove members.
 * Grant a team access on the Access page and every member inherits it. Gated on
 * `whoami.can_admin`.
 */
export function Teams() {
  return (
    <AdminGuard title='Teams' sub='Groups that grants can target'>
      <TeamsBody />
    </AdminGuard>
  )
}

function TeamsBody() {
  const { org } = useScope()
  const { data: teams = [] } = useTeams(org)
  const [createOpen, setCreateOpen] = useState(false)

  return (
    <>
      <PageHeader title='Teams' sub='Groups that grants can target' />
      <Main fluid fixed className='flex min-h-0 flex-col'>
        <div className='mb-3 flex items-center justify-between'>
          <p className='text-sm text-muted-foreground'>
            {teams.length} {teams.length === 1 ? 'team' : 'teams'}
          </p>
          <Button size='sm' onClick={() => setCreateOpen(true)} disabled={!org}>
            <Plus className='size-4' /> New team
          </Button>
        </div>

        <div className='scroll min-h-0 flex-1 space-y-4 overflow-y-auto pe-1'>
          {teams.length === 0 ? (
            <Card className='grid h-40 place-items-center'>
              <p className='text-sm text-muted-foreground'>No teams yet.</p>
            </Card>
          ) : (
            teams.map((t) => <TeamCard key={t.id} org={org as string} team={t} />)
          )}
        </div>
      </Main>

      {org ? (
        <TeamFormDialog org={org} open={createOpen} onOpenChange={setCreateOpen} />
      ) : null}
    </>
  )
}

function TeamCard({ org, team }: { org: string; team: Team }) {
  const { data: members = [] } = useTeamMembers(org, team.id)
  const { data: users = [] } = useUsers(org)
  const del = useDeleteTeam(org)
  const add = useAddTeamMember(org, team.id)
  const remove = useRemoveTeamMember(org, team.id)
  const [confirmOpen, setConfirmOpen] = useState(false)
  const [addUser, setAddUser] = useState<string>('')

  const memberIds = new Set(members.map((m) => m.id))
  const candidates = users.filter((u) => !memberIds.has(u.id))

  return (
    <Card className='p-3'>
      <div className='mb-2.5 flex items-center justify-between'>
        <div className='flex items-center gap-2'>
          <Users className='size-4 text-primary' />
          <span className='text-sm font-medium'>{team.name}</span>
          <code className='text-[11px] text-muted-foreground'>{team.slug}</code>
          <Badge variant='muted' className='h-4 px-1.5 text-[10px]'>
            {members.length} {members.length === 1 ? 'member' : 'members'}
          </Badge>
        </div>
        <Button
          size='icon'
          variant='ghost'
          className='size-7 text-sev-fault'
          onClick={() => setConfirmOpen(true)}
        >
          <Trash2 className='size-3.5' />
        </Button>
      </div>

      <div className='space-y-1.5'>
        {members.map((m) => (
          <MemberPill
            key={m.id}
            user={m}
            onRemove={() => remove.mutate(m.id)}
            removing={remove.isPending}
          />
        ))}
      </div>

      <div className='mt-2.5 flex items-center gap-2'>
        <Select value={addUser} onValueChange={setAddUser}>
          <SelectTrigger className='h-8 flex-1'>
            <SelectValue placeholder='Add a member…' />
          </SelectTrigger>
          <SelectContent>
            {candidates.length === 0 ? (
              <SelectItem value='__none' disabled>
                No more users to add
              </SelectItem>
            ) : (
              candidates.map((u) => (
                <SelectItem key={u.id} value={u.id}>
                  {u.display_name} ({u.email})
                </SelectItem>
              ))
            )}
          </SelectContent>
        </Select>
        <Button
          size='sm'
          variant='outline'
          disabled={!addUser || addUser === '__none' || add.isPending}
          onClick={() =>
            add.mutate(addUser, { onSuccess: () => setAddUser('') })
          }
        >
          Add
        </Button>
      </div>

      <ConfirmDialog
        open={confirmOpen}
        onOpenChange={setConfirmOpen}
        destructive
        title={`Delete team ${team.name}?`}
        desc={
          <>
            This removes the team and every membership. Grants targeting this team
            stop applying.
          </>
        }
        confirmText='Delete team'
        isLoading={del.isPending}
        handleConfirm={() =>
          del.mutate(team.id, { onSuccess: () => setConfirmOpen(false) })
        }
      />
    </Card>
  )
}

function MemberPill({
  user,
  onRemove,
  removing,
}: {
  user: User
  onRemove: () => void
  removing: boolean
}) {
  return (
    <div className='flex items-center justify-between rounded-md bg-muted/40 px-2.5 py-1.5'>
      <div className='flex items-center gap-2'>
        <span className='text-sm'>{user.display_name}</span>
        <code className='text-[10px] text-muted-foreground'>{user.email}</code>
      </div>
      <Button
        size='icon'
        variant='ghost'
        className='size-6'
        disabled={removing}
        onClick={onRemove}
      >
        <X className='size-3.5' />
      </Button>
    </div>
  )
}

function TeamFormDialog({
  org,
  open,
  onOpenChange,
}: {
  org: string
  open: boolean
  onOpenChange: (open: boolean) => void
}) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className='sm:max-w-md'>
        {open ? <TeamFormBody org={org} onOpenChange={onOpenChange} /> : null}
      </DialogContent>
    </Dialog>
  )
}

function TeamFormBody({
  org,
  onOpenChange,
}: {
  org: string
  onOpenChange: (open: boolean) => void
}) {
  const create = useCreateTeam(org)
  const [slug, setSlug] = useState('')
  const [name, setName] = useState('')
  const [error, setError] = useState<string | null>(null)

  const submit = () => {
    if (!slug.trim() || !name.trim()) {
      setError('Slug and name are required.')
      return
    }
    create.mutate(
      { slug: slug.trim(), name: name.trim() },
      {
        onSuccess: () => onOpenChange(false),
        onError: (e) => setError((e as Error).message),
      }
    )
  }

  return (
    <>
      <DialogHeader>
        <DialogTitle>New team</DialogTitle>
      </DialogHeader>
      <div className='space-y-3 py-1'>
        <div className='space-y-1.5'>
          <Label className='text-[12px]'>Slug</Label>
          <Input
            value={slug}
            onChange={(e) => setSlug(e.target.value)}
            placeholder='ops'
          />
        </div>
        <div className='space-y-1.5'>
          <Label className='text-[12px]'>Name</Label>
          <Input
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder='Operations'
          />
        </div>
        {error ? <p className='text-[12px] text-sev-fault'>{error}</p> : null}
      </div>
      <DialogFooter>
        <Button variant='ghost' onClick={() => onOpenChange(false)}>
          Cancel
        </Button>
        <Button onClick={submit} disabled={create.isPending}>
          {create.isPending ? 'Saving…' : 'Create'}
        </Button>
      </DialogFooter>
    </>
  )
}
