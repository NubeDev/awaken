/**
 * Sites admin landing (DOMAIN-MODEL §site): every `kind:"site"` with its parent
 * tenant, address, timezone and a rolled-up gateway-count. Create / edit / delete
 * a site via site-form.tsx. All reads/writes go through the rubix records API.
 */
import { useState } from 'react'
import { Pencil, Plus, Trash2 } from 'lucide-react'
import type { SiteRecord } from '@/api/records'
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
import { useDeleteSite, useGateways, useSites, useTenants } from './hooks'
import { SiteForm } from './site-form'
import { SiteMap } from './site-map'

export function SiteList() {
  const sites = useSites()
  const tenants = useTenants()
  const gateways = useGateways()
  const del = useDeleteSite()

  const [adding, setAdding] = useState(false)
  const [editing, setEditing] = useState<SiteRecord | null>(null)
  const [deleteTarget, setDeleteTarget] = useState<SiteRecord | null>(null)

  const rows = sites.data ?? []
  const allTenants = tenants.data ?? []
  const allGateways = gateways.data ?? []
  // Resolve a site's parent-tenant record id to a display label.
  const tenantLabel = (id?: string) => {
    const t = allTenants.find((t) => t.id === id)
    return t ? `${t.content.name}` : '—'
  }

  return (
    <div className='space-y-4'>
      <div className='flex items-center justify-between'>
        <div>
          <h2 className='text-xl font-semibold'>Sites</h2>
          <p className='text-muted-foreground text-sm'>
            Physical locations under a tenant. Each site hosts gateways; its
            timezone drives site-local time on dashboards.
          </p>
        </div>
        <Button onClick={() => setAdding(true)}>
          <Plus className='mr-1 size-4' /> New site
        </Button>
      </div>

      <SiteMap />

      <Card className='overflow-x-auto p-0'>
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Key</TableHead>
              <TableHead>Name</TableHead>
              <TableHead>Tenant</TableHead>
              <TableHead>Address</TableHead>
              <TableHead>Timezone</TableHead>
              <TableHead className='text-center'>Gateways</TableHead>
              <TableHead className='text-right'>Actions</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {sites.isLoading ? (
              <TableRow>
                <TableCell colSpan={7} className='text-muted-foreground'>
                  Loading…
                </TableCell>
              </TableRow>
            ) : rows.length === 0 ? (
              <TableRow>
                <TableCell colSpan={7} className='text-muted-foreground'>
                  No sites yet.
                </TableCell>
              </TableRow>
            ) : (
              rows.map((s) => {
                const gwCount = allGateways.filter(
                  (g) => g.content.site === s.id
                ).length
                return (
                  <TableRow key={s.id}>
                    <TableCell className='font-mono text-xs'>
                      {s.content.key}
                    </TableCell>
                    <TableCell>{s.content.name}</TableCell>
                    <TableCell>{tenantLabel(s.content.tenant)}</TableCell>
                    <TableCell className='text-muted-foreground'>
                      {s.content.address ?? '—'}
                    </TableCell>
                    <TableCell className='font-mono text-xs'>
                      {s.content.timezone ?? '—'}
                    </TableCell>
                    <TableCell className='text-center'>{gwCount}</TableCell>
                    <TableCell className='text-right'>
                      <div className='flex justify-end gap-1'>
                        <Button
                          variant='ghost'
                          size='icon'
                          title='Edit'
                          onClick={() => setEditing(s)}
                        >
                          <Pencil className='size-4' />
                        </Button>
                        <Button
                          variant='ghost'
                          size='icon'
                          title='Delete'
                          onClick={() => setDeleteTarget(s)}
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
        <SiteForm open onOpenChange={(o) => !o && setAdding(false)} />
      ) : null}

      {editing ? (
        <SiteForm
          open
          onOpenChange={(o) => !o && setEditing(null)}
          site={editing}
        />
      ) : null}

      {deleteTarget ? (
        <ConfirmDialog
          open
          onOpenChange={(o) => !o && setDeleteTarget(null)}
          destructive
          title={`Delete ${deleteTarget.content.name}?`}
          desc='This removes the site. Its gateways, networks and meters are not cascaded — remove them first.'
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
