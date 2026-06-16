/**
 * Users & service-accounts admin (ADMIN.md §6): every principal in the tenant —
 * users (`kind:"user"`) and service accounts (`kind:"extension"`, e.g. the poller
 * `agent`) — with its NHP role (inline-editable, role-select.tsx), its grants
 * (read-only, grants.tsx), create (edit.tsx) and delete. Wired to the rubix
 * principal/grant admin API (api/admin.ts) as the seeded admin, NOT mocked.
 *
 * The poller service account (`agent`, kind extension) is shown like any
 * principal but flagged so it isn't mistaken for a human user; its role/delete are
 * left available to admins (it's a real managed principal) but it carries a hint.
 */
import { useState } from 'react'
import { KeyRound, Plus, Trash2 } from 'lucide-react'
import type { Principal } from '@/api/admin'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card } from '@/components/ui/card'
import { ConfirmDialog } from '@/components/confirm-dialog'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import { UserForm } from './edit'
import { GrantsDialog } from './grants'
import { useDeletePrincipal, usePrincipals } from './hooks'
import { RoleSelect } from './role-select'

export function UserList() {
  const principals = usePrincipals()
  const del = useDeletePrincipal()

  const [adding, setAdding] = useState(false)
  const [grantsFor, setGrantsFor] = useState<string | null>(null)
  const [deleteTarget, setDeleteTarget] = useState<Principal | null>(null)

  const rows = principals.data ?? []

  return (
    <div className='space-y-4'>
      <div className='flex items-center justify-between'>
        <div>
          <h2 className='text-xl font-semibold'>Users &amp; service accounts</h2>
          <p className='text-muted-foreground text-sm'>
            Principals in this tenant. Users and the polling service account share
            one identity model; roles map to NHP access (ADMIN §6).
          </p>
        </div>
        <Button onClick={() => setAdding(true)}>
          <Plus className='mr-1 size-4' /> New principal
        </Button>
      </div>

      {principals.isError ? (
        <Card className='border-amber-500/40 p-4 text-sm'>
          Could not load principals: {(principals.error as Error).message}. The
          principal API is admin-only — set <code>VITE_RUBIX_ADMIN_SUBJECT</code> /{' '}
          <code>VITE_RUBIX_ADMIN_SECRET</code> to a namespace admin (see
          WS-05.md).
        </Card>
      ) : null}

      <Card className='overflow-x-auto p-0'>
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Subject</TableHead>
              <TableHead>Kind</TableHead>
              <TableHead>Role</TableHead>
              <TableHead className='text-right'>Actions</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {principals.isLoading ? (
              <TableRow>
                <TableCell colSpan={4} className='text-muted-foreground'>
                  Loading…
                </TableCell>
              </TableRow>
            ) : rows.length === 0 ? (
              <TableRow>
                <TableCell colSpan={4} className='text-muted-foreground'>
                  No principals.
                </TableCell>
              </TableRow>
            ) : (
              rows.map((p) => (
                <TableRow key={p.subject}>
                  <TableCell className='font-mono text-xs'>
                    {p.subject}
                  </TableCell>
                  <TableCell>
                    {p.kind === 'extension' ? (
                      <Badge variant='outline' title='Service account (e.g. the poller)'>
                        service account
                      </Badge>
                    ) : (
                      'user'
                    )}
                  </TableCell>
                  <TableCell>
                    <RoleSelect subject={p.subject} role={p.role} />
                  </TableCell>
                  <TableCell className='text-right'>
                    <div className='flex justify-end gap-1'>
                      <Button
                        variant='ghost'
                        size='icon'
                        title='Grants'
                        onClick={() => setGrantsFor(p.subject)}
                      >
                        <KeyRound className='size-4' />
                      </Button>
                      <Button
                        variant='ghost'
                        size='icon'
                        title='Delete'
                        onClick={() => setDeleteTarget(p)}
                      >
                        <Trash2 className='size-4' />
                      </Button>
                    </div>
                  </TableCell>
                </TableRow>
              ))
            )}
          </TableBody>
        </Table>
      </Card>

      {adding ? (
        <UserForm open onOpenChange={(o) => !o && setAdding(false)} />
      ) : null}

      {grantsFor ? (
        <GrantsDialog
          subject={grantsFor}
          open
          onOpenChange={(o) => !o && setGrantsFor(null)}
        />
      ) : null}

      {deleteTarget ? (
        <ConfirmDialog
          open
          onOpenChange={(o) => !o && setDeleteTarget(null)}
          destructive
          title={`Delete ${deleteTarget.subject}?`}
          desc='This removes the principal and its credential. The last admin in a tenant cannot be deleted.'
          confirmText='Delete'
          isLoading={del.isPending}
          handleConfirm={() =>
            del.mutate(deleteTarget.subject, {
              onSuccess: () => setDeleteTarget(null),
            })
          }
        />
      ) : null}
    </div>
  )
}
