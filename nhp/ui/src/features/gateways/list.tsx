/**
 * Gateways admin landing (ADMIN.md §5): every `kind:"gateway"` with its
 * poller-written online status + last-seen (read-only), and an expandable panel of
 * the gateway's networks (network-table.tsx) with each network's device-count vs
 * cap. Create / edit / delete a gateway via gateway-form.tsx. All reads/writes go
 * through the rubix records API; status/last_seen are display-only.
 */
import { Fragment, useState } from 'react'
import { ChevronDown, ChevronRight, Pencil, Plus, Trash2 } from 'lucide-react'
import type { GatewayRecord } from '@/api/records'
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
import { GatewayForm } from './gateway-form'
import { useDeleteGateway, useGateways, useMeters, useNetworks } from './hooks'
import { NetworkTable } from './network-table'
import { StatusBadge } from './status-badge'

export function GatewayList() {
  const gateways = useGateways()
  const networks = useNetworks()
  const meters = useMeters()
  const del = useDeleteGateway()

  const [expanded, setExpanded] = useState<string | null>(null)
  const [adding, setAdding] = useState(false)
  const [editing, setEditing] = useState<GatewayRecord | null>(null)
  const [deleteTarget, setDeleteTarget] = useState<GatewayRecord | null>(null)

  const rows = gateways.data ?? []
  const allNetworks = networks.data ?? []
  const allMeters = meters.data ?? []

  return (
    <div className='space-y-4'>
      <div className='flex items-center justify-between'>
        <div>
          <h2 className='text-xl font-semibold'>Gateways</h2>
          <p className='text-muted-foreground text-sm'>
            Field devices and their networks. Online status and last-seen are
            written by the polling service (read-only here).
          </p>
        </div>
        <Button onClick={() => setAdding(true)}>
          <Plus className='mr-1 size-4' /> New gateway
        </Button>
      </div>

      <Card className='overflow-x-auto p-0'>
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead className='w-8' />
              <TableHead>Key</TableHead>
              <TableHead>Name</TableHead>
              <TableHead>Model</TableHead>
              <TableHead>Host</TableHead>
              <TableHead>Status</TableHead>
              <TableHead className='text-center'>Networks</TableHead>
              <TableHead className='text-right'>Actions</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {gateways.isLoading ? (
              <TableRow>
                <TableCell colSpan={8} className='text-muted-foreground'>
                  Loading…
                </TableCell>
              </TableRow>
            ) : rows.length === 0 ? (
              <TableRow>
                <TableCell colSpan={8} className='text-muted-foreground'>
                  No gateways yet.
                </TableCell>
              </TableRow>
            ) : (
              rows.map((g) => {
                const gwNetworks = allNetworks.filter(
                  (n) => n.content.gateway === g.id
                )
                const isOpen = expanded === g.id
                return (
                  <Fragment key={g.id}>
                    <TableRow>
                      <TableCell>
                        <Button
                          variant='ghost'
                          size='icon'
                          onClick={() => setExpanded(isOpen ? null : g.id)}
                          title={isOpen ? 'Collapse' : 'Expand networks'}
                        >
                          {isOpen ? (
                            <ChevronDown className='size-4' />
                          ) : (
                            <ChevronRight className='size-4' />
                          )}
                        </Button>
                      </TableCell>
                      <TableCell className='font-mono text-xs'>
                        {g.content.key}
                      </TableCell>
                      <TableCell>{g.content.name}</TableCell>
                      <TableCell>{g.content.model ?? '—'}</TableCell>
                      <TableCell className='font-mono text-xs'>
                        {g.content.host ?? '—'}
                      </TableCell>
                      <TableCell>
                        <StatusBadge
                          status={g.content.status}
                          lastSeen={g.content.last_seen}
                        />
                      </TableCell>
                      <TableCell className='text-center'>
                        {gwNetworks.length}
                      </TableCell>
                      <TableCell className='text-right'>
                        <div className='flex justify-end gap-1'>
                          <Button
                            variant='ghost'
                            size='icon'
                            title='Edit'
                            onClick={() => setEditing(g)}
                          >
                            <Pencil className='size-4' />
                          </Button>
                          <Button
                            variant='ghost'
                            size='icon'
                            title='Delete'
                            onClick={() => setDeleteTarget(g)}
                          >
                            <Trash2 className='size-4' />
                          </Button>
                        </div>
                      </TableCell>
                    </TableRow>
                    {isOpen ? (
                      <TableRow>
                        <TableCell colSpan={8} className='bg-muted/30'>
                          <NetworkTable
                            gateway={g}
                            networks={gwNetworks}
                            meters={allMeters}
                          />
                        </TableCell>
                      </TableRow>
                    ) : null}
                  </Fragment>
                )
              })
            )}
          </TableBody>
        </Table>
      </Card>

      {adding ? (
        <GatewayForm open onOpenChange={(o) => !o && setAdding(false)} />
      ) : null}

      {editing ? (
        <GatewayForm
          open
          onOpenChange={(o) => !o && setEditing(null)}
          gateway={editing}
        />
      ) : null}

      {deleteTarget ? (
        <ConfirmDialog
          open
          onOpenChange={(o) => !o && setDeleteTarget(null)}
          destructive
          title={`Delete ${deleteTarget.content.name}?`}
          desc='This removes the gateway. Its networks and meters are not cascaded — remove them first.'
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
