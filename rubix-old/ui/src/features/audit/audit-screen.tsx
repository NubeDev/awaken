/**
 * Admin Audit log screen (docs/design/audit-and-undo.md "Audit read surface"):
 * the org's change ledger with the server's filters (kind / resource_id / actor /
 * op / limit), newest-first, with before→after diff rendering. Gated behind the
 * audit capability — the screen reuses `AdminGuard` (whoami.can_admin), matching
 * the server's `require_admin(org)` on `GET /api/v1/audit`, so a non-admin sees a
 * clear "not authorized" panel rather than a wall of 403s.
 */
import { useState } from 'react'
import { useAudit } from '@/api/hooks'
import type { AuditQuery, Op } from '@/api/types'
import { useScope } from '@/context/scope-provider'
import { Card } from '@/components/ui/card'
import { Input } from '@/components/ui/input'
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
import { AdminGuard } from '@/features/admin/admin-guard'
import { ChangeTimeline } from './components/change-timeline'

const OPS: { value: Op | 'all'; label: string }[] = [
  { value: 'all', label: 'All operations' },
  { value: 'create', label: 'Create' },
  { value: 'update', label: 'Update' },
  { value: 'delete', label: 'Delete' },
]

export function AuditScreen() {
  return (
    <AdminGuard title='Audit log' sub='Every recorded change in this org'>
      <AuditBody />
    </AdminGuard>
  )
}

function AuditBody() {
  const { org } = useScope()
  const [kind, setKind] = useState('')
  const [resourceId, setResourceId] = useState('')
  const [actor, setActor] = useState('')
  const [op, setOp] = useState<Op | 'all'>('all')

  const filter: Omit<AuditQuery, 'org'> = {
    kind: kind.trim() || undefined,
    resource_id: resourceId.trim() || undefined,
    actor: actor.trim() || undefined,
    op: op === 'all' ? undefined : op,
    limit: 200,
  }

  const { data = [], isLoading, isError, error } = useAudit(org, filter)

  return (
    <>
      <PageHeader title='Audit log' sub='Every recorded change in this org' />
      <Main fluid fixed className='flex min-h-0 flex-col'>
        <div className='mb-3 grid gap-3 sm:grid-cols-2 lg:grid-cols-4'>
          <div className='space-y-1.5'>
            <Label className='text-[12px]'>Kind</Label>
            <Input
              value={kind}
              onChange={(e) => setKind(e.target.value)}
              placeholder='dashboard, rule, …'
            />
          </div>
          <div className='space-y-1.5'>
            <Label className='text-[12px]'>Resource id</Label>
            <Input
              value={resourceId}
              onChange={(e) => setResourceId(e.target.value)}
              placeholder='uuid'
            />
          </div>
          <div className='space-y-1.5'>
            <Label className='text-[12px]'>Actor (subject)</Label>
            <Input
              value={actor}
              onChange={(e) => setActor(e.target.value)}
              placeholder='subject or run id'
            />
          </div>
          <div className='space-y-1.5'>
            <Label className='text-[12px]'>Operation</Label>
            <Select value={op} onValueChange={(v) => setOp(v as Op | 'all')}>
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {OPS.map((o) => (
                  <SelectItem key={o.value} value={o.value}>
                    {o.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
        </div>

        <div className='scroll min-h-0 flex-1 overflow-y-auto pe-1'>
          {isLoading ? (
            <Card className='grid h-40 place-items-center'>
              <p className='text-sm text-muted-foreground'>Loading…</p>
            </Card>
          ) : isError ? (
            <Card className='grid h-40 place-items-center'>
              <p className='text-sm text-sev-fault'>{(error as Error).message}</p>
            </Card>
          ) : (
            <ChangeTimeline
              changes={data}
              showKind
              emptyLabel='No changes match these filters.'
            />
          )}
        </div>
      </Main>
    </>
  )
}
