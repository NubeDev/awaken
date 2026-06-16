/**
 * Tenants admin landing (DOMAIN-MODEL §tenant): every `kind:"tenant"` with its
 * namespace and a rolled-up site-count. Create / edit / delete a tenant via
 * tenant-form.tsx. All reads/writes go through the rubix records API.
 */
import { useState } from 'react'
import { Pencil, Plus, Trash2 } from 'lucide-react'
import type { TenantRecord } from '@/api/records'
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
import { useDeleteTenant, useSites, useTenants } from './hooks'
import { TenantForm } from './tenant-form'

export function TenantList() {
  const tenants = useTenants()
  const sites = useSites()
  const del = useDeleteTenant()

  const [adding, setAdding] = useState(false)
  const [editing, setEditing] = useState<TenantRecord | null>(null)
  const [deleteTarget, setDeleteTarget] = useState<TenantRecord | null>(null)

  const rows = tenants.data ?? []
  const allSites = sites.data ?? []

  return (
    <div className='space-y-4'>
      <div className='flex items-center justify-between'>
        <div>
          <h2 className='text-xl font-semibold'>Tenants</h2>
          <p className='text-muted-foreground text-sm'>
            The portfolio root. Each tenant owns sites, which own gateways,
            networks and meters.
          </p>
        </div>
        <Button onClick={() => setAdding(true)}>
          <Plus className='mr-1 size-4' /> New tenant
        </Button>
      </div>

      <Card className='overflow-x-auto p-0'>
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Key</TableHead>
              <TableHead>Name</TableHead>
              <TableHead>Namespace</TableHead>
              <TableHead className='text-center'>Sites</TableHead>
              <TableHead className='text-right'>Actions</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {tenants.isLoading ? (
              <TableRow>
                <TableCell colSpan={5} className='text-muted-foreground'>
                  Loading…
                </TableCell>
              </TableRow>
            ) : rows.length === 0 ? (
              <TableRow>
                <TableCell colSpan={5} className='text-muted-foreground'>
                  No tenants yet.
                </TableCell>
              </TableRow>
            ) : (
              rows.map((t) => {
                const siteCount = allSites.filter(
                  (s) => s.content.tenant === t.id
                ).length
                return (
                  <TableRow key={t.id}>
                    <TableCell className='font-mono text-xs'>
                      {t.content.key}
                    </TableCell>
                    <TableCell>{t.content.name}</TableCell>
                    <TableCell className='font-mono text-xs'>
                      {t.content.namespace ?? '—'}
                    </TableCell>
                    <TableCell className='text-center'>{siteCount}</TableCell>
                    <TableCell className='text-right'>
                      <div className='flex justify-end gap-1'>
                        <Button
                          variant='ghost'
                          size='icon'
                          title='Edit'
                          onClick={() => setEditing(t)}
                        >
                          <Pencil className='size-4' />
                        </Button>
                        <Button
                          variant='ghost'
                          size='icon'
                          title='Delete'
                          onClick={() => setDeleteTarget(t)}
                        >
                          <Trash2 className='size-4' />
                        </Button>
                      </div>
                    </TableCell>
                  </TableRow>
                )
              })
            )}
          </TableBody>
        </Table>
      </Card>

      {adding ? (
        <TenantForm open onOpenChange={(o) => !o && setAdding(false)} />
      ) : null}

      {editing ? (
        <TenantForm
          open
          onOpenChange={(o) => !o && setEditing(null)}
          tenant={editing}
        />
      ) : null}

      {deleteTarget ? (
        <ConfirmDialog
          open
          onOpenChange={(o) => !o && setDeleteTarget(null)}
          destructive
          title={`Delete ${deleteTarget.content.name}?`}
          desc='This removes the tenant. Its sites and their devices are not cascaded — remove them first.'
          confirmText='Delete'
          isLoading={del.isPending}
          handleConfirm={() =>
            del.mutate(deleteTarget.id, {
              onSuccess: () => setDeleteTarget(null),
            })
          }
        />
      ) : null}
    </div>
  )
}
