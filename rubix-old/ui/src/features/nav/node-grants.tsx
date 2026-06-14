/**
 * Inline grant management for one nav node (docs/design/page-context-and-nav.md
 * §6): grant a team/user `view` (read) or `edit` (write) on the node, reusing
 * the existing generic grant model (`nav_node:<id>` resource ref) rather than a
 * parallel one. Navigation access is the node's grant — `view`-to-navigate, not
 * the board's grant — so this is where the sidebar's per-node visibility is set.
 */
import { useMemo, useState } from 'react'
import { Trash2 } from 'lucide-react'
import {
  useCreateGrant,
  useDeleteGrant,
  useGrants,
  useTeams,
  useUsers,
} from '@/api/hooks'
import type { Permission, SubjectKind } from '@/api/types'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'

export function NodeGrants({ org, nodeId }: { org: string; nodeId: string }) {
  const resourceRef = `nav_node:${nodeId}`
  const { data: grants = [] } = useGrants(org, resourceRef)
  const { data: teams = [] } = useTeams(org)
  const { data: users = [] } = useUsers(org)
  const create = useCreateGrant(org)
  const del = useDeleteGrant(org)

  const [kind, setKind] = useState<SubjectKind>('team')
  const [subjectId, setSubjectId] = useState('')
  const [permission, setPermission] = useState<Permission>('read')

  const subjectName = useMemo(() => {
    const m = new Map<string, string>()
    for (const t of teams) m.set(t.id, `Team · ${t.name}`)
    for (const u of users) m.set(u.id, `User · ${u.display_name}`)
    return m
  }, [teams, users])

  const candidates = kind === 'team' ? teams : users
  const nodeGrants = grants.filter(
    (g) => g.resource_kind === 'nav_node' && g.resource_ref === resourceRef
  )

  const add = () => {
    if (!subjectId || subjectId === '__none') return
    create.mutate(
      {
        subject_kind: kind,
        subject_id: subjectId,
        resource_kind: 'nav_node',
        resource_ref: resourceRef,
        permission,
      },
      { onSuccess: () => setSubjectId('') }
    )
  }

  return (
    <div className='space-y-2 rounded-md border bg-muted/30 p-2'>
      <p className='text-[11px] font-medium text-muted-foreground'>
        Access (view = navigate; edit = manage)
      </p>
      <div className='flex flex-wrap items-end gap-1.5'>
        <Select
          value={kind}
          onValueChange={(v) => {
            setKind(v as SubjectKind)
            setSubjectId('')
          }}
        >
          <SelectTrigger size='sm' className='w-24'>
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value='team'>Team</SelectItem>
            <SelectItem value='user'>User</SelectItem>
          </SelectContent>
        </Select>
        <Select value={subjectId} onValueChange={setSubjectId}>
          <SelectTrigger size='sm' className='w-40'>
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
                    : (c as { display_name: string }).display_name}
                </SelectItem>
              ))
            )}
          </SelectContent>
        </Select>
        <Select
          value={permission}
          onValueChange={(v) => setPermission(v as Permission)}
        >
          <SelectTrigger size='sm' className='w-24'>
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value='read'>View</SelectItem>
            <SelectItem value='write'>Edit</SelectItem>
          </SelectContent>
        </Select>
        <Button
          size='sm'
          variant='outline'
          disabled={!subjectId || subjectId === '__none' || create.isPending}
          onClick={add}
        >
          Grant
        </Button>
      </div>
      {nodeGrants.length > 0 ? (
        <div className='flex flex-wrap gap-1.5'>
          {nodeGrants.map((g) => (
            <Badge key={g.id} variant='outline' className='gap-1 font-normal'>
              {subjectName.get(g.subject_id) ?? g.subject_id} ·{' '}
              {g.permission === 'read' ? 'view' : 'edit'}
              <button
                className='text-sev-fault'
                onClick={() => del.mutate(g.id)}
                title='Revoke'
              >
                <Trash2 className='size-3' />
              </button>
            </Badge>
          ))}
        </div>
      ) : null}
    </div>
  )
}
