import { useMemo, useState } from 'react'
import { KeyRound, Trash2 } from 'lucide-react'
import {
  useDashboardGrants,
  useDashboards,
  useDeleteGrant,
  useGrantDashboard,
  useTeams,
  useUsers,
} from '@/api/hooks'
import type { Grant, Permission, SubjectKind, Uuid } from '@/api/types'
import { useScope } from '@/context/scope-provider'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card } from '@/components/ui/card'
import { Label } from '@/components/ui/label'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { Main } from '@/components/layout/main'
import { PageHeader } from '@/components/layout/page-header'
import { AdminGuard } from './admin-guard'

/**
 * Access: per-resource grants. Pick a dashboard, then grant a team or user
 * read/write on it — the Grafana/Niagara "select exactly what each user or team
 * can see and edit" model. Grants ADD access on top of scope-role. Gated on
 * `whoami.can_admin`. See docs/design/authz-rbac.md increment E.
 */
export function Access() {
  return (
    <AdminGuard title='Access' sub='Per-dashboard read/write grants'>
      <AccessBody />
    </AdminGuard>
  )
}

function AccessBody() {
  const { org } = useScope()
  const { data: dashboards = [] } = useDashboards(org)
  const [dashId, setDashId] = useState<Uuid | ''>('')

  return (
    <>
      <PageHeader title='Access' sub='Per-dashboard read/write grants' />
      <Main fluid fixed className='flex min-h-0 flex-col'>
        <div className='mb-3 max-w-md space-y-1.5'>
          <Label className='text-[12px]'>Dashboard</Label>
          <Select value={dashId} onValueChange={(v) => setDashId(v as Uuid)}>
            <SelectTrigger>
              <SelectValue placeholder='Pick a dashboard…' />
            </SelectTrigger>
            <SelectContent>
              {dashboards.length === 0 ? (
                <SelectItem value='__none' disabled>
                  No dashboards in this org
                </SelectItem>
              ) : (
                dashboards.map((d) => (
                  <SelectItem key={d.id} value={d.id}>
                    {d.title} ({d.slug})
                  </SelectItem>
                ))
              )}
            </SelectContent>
          </Select>
        </div>

        {dashId && org ? (
          <DashboardAccess org={org} dashboardId={dashId} />
        ) : (
          <Card className='grid h-40 place-items-center'>
            <p className='text-sm text-muted-foreground'>
              Pick a dashboard to manage its grants.
            </p>
          </Card>
        )}
      </Main>
    </>
  )
}

function DashboardAccess({ org, dashboardId }: { org: string; dashboardId: Uuid }) {
  const { data: grants = [] } = useDashboardGrants(dashboardId)
  const { data: teams = [] } = useTeams(org)
  const { data: users = [] } = useUsers(org)
  const grant = useGrantDashboard(dashboardId)
  const del = useDeleteGrant(org)

  const [kind, setKind] = useState<SubjectKind>('team')
  const [subjectId, setSubjectId] = useState<string>('')
  const [permission, setPermission] = useState<Permission>('read')

  const subjectName = useMemo(() => {
    const m = new Map<string, string>()
    for (const t of teams) m.set(t.id, `Team · ${t.name}`)
    for (const u of users) m.set(u.id, `User · ${u.display_name}`)
    return m
  }, [teams, users])

  const candidates = kind === 'team' ? teams : users

  const addGrant = () => {
    if (!subjectId) return
    grant.mutate(
      { subject_kind: kind, subject_id: subjectId, permission },
      { onSuccess: () => setSubjectId('') }
    )
  }

  return (
    <div className='scroll min-h-0 flex-1 space-y-4 overflow-y-auto pe-1'>
      <Card className='space-y-3 p-3'>
        <p className='text-sm font-medium'>Grant access</p>
        <div className='flex flex-wrap items-end gap-2'>
          <div className='space-y-1.5'>
            <Label className='text-[11px]'>Subject</Label>
            <Select
              value={kind}
              onValueChange={(v) => {
                setKind(v as SubjectKind)
                setSubjectId('')
              }}
            >
              <SelectTrigger className='h-8 w-28'>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value='team'>Team</SelectItem>
                <SelectItem value='user'>User</SelectItem>
              </SelectContent>
            </Select>
          </div>
          <div className='min-w-48 flex-1 space-y-1.5'>
            <Label className='text-[11px]'>{kind === 'team' ? 'Team' : 'User'}</Label>
            <Select value={subjectId} onValueChange={setSubjectId}>
              <SelectTrigger className='h-8'>
                <SelectValue placeholder={`Pick a ${kind}…`} />
              </SelectTrigger>
              <SelectContent>
                {candidates.length === 0 ? (
                  <SelectItem value='__none' disabled>
                    None available
                  </SelectItem>
                ) : (
                  candidates.map((c) => (
                    <SelectItem key={c.id} value={c.id}>
                      {kind === 'team'
                        ? (c as { name: string }).name
                        : `${(c as { display_name: string }).display_name}`}
                    </SelectItem>
                  ))
                )}
              </SelectContent>
            </Select>
          </div>
          <div className='space-y-1.5'>
            <Label className='text-[11px]'>Permission</Label>
            <Select
              value={permission}
              onValueChange={(v) => setPermission(v as Permission)}
            >
              <SelectTrigger className='h-8 w-28'>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value='read'>Read</SelectItem>
                <SelectItem value='write'>Write</SelectItem>
              </SelectContent>
            </Select>
          </div>
          <Button
            size='sm'
            disabled={!subjectId || subjectId === '__none' || grant.isPending}
            onClick={addGrant}
          >
            Grant
          </Button>
        </div>
      </Card>

      <Card className='p-3'>
        <p className='mb-2 text-sm font-medium'>
          {grants.length} {grants.length === 1 ? 'grant' : 'grants'}
        </p>
        {grants.length === 0 ? (
          <p className='text-sm text-muted-foreground'>
            No grants on this dashboard yet.
          </p>
        ) : (
          <div className='space-y-1.5'>
            {grants.map((g) => (
              <GrantRow
                key={g.id}
                grant={g}
                label={
                  subjectName.get(g.subject_id) ??
                  `${g.subject_kind} · ${g.subject_id}`
                }
                onRemove={() => del.mutate(g.id)}
                removing={del.isPending}
              />
            ))}
          </div>
        )}
      </Card>
    </div>
  )
}

function GrantRow({
  grant,
  label,
  onRemove,
  removing,
}: {
  grant: Grant
  label: string
  onRemove: () => void
  removing: boolean
}) {
  return (
    <div className='flex items-center justify-between rounded-md bg-muted/40 px-3 py-2'>
      <div className='flex items-center gap-2'>
        <KeyRound className='size-4 text-primary' />
        <span className='text-sm'>{label}</span>
        <Badge
          variant={grant.permission === 'write' ? 'default' : 'muted'}
          className='h-4 px-1.5 text-[10px]'
        >
          {grant.permission}
        </Badge>
      </div>
      <Button
        size='icon'
        variant='ghost'
        className='size-7 text-sev-fault'
        disabled={removing}
        onClick={onRemove}
      >
        <Trash2 className='size-3.5' />
      </Button>
    </div>
  )
}
