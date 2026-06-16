/**
 * Meters admin landing (DOMAIN-MODEL §meter): every `kind:"meter"` with its
 * resolved network + meter-type, the stamped type version, Modbus address, and
 * its poller-written online status + last-seen (read-only). Meters are stamped by
 * the meters-wizard (WS-06), so this surface is browse + delete only. All reads
 * go through the rubix records API.
 */
import { useState } from 'react'
import { Trash2 } from 'lucide-react'
import type { MeterRecord } from '@/api/records'
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
import { StatusBadge } from '@/features/gateways/status-badge'
import { useDeleteMeter, useMeterTypes, useMeters, useNetworks } from './hooks'

export function MeterList() {
  const meters = useMeters()
  const networks = useNetworks()
  const meterTypes = useMeterTypes()
  const del = useDeleteMeter()

  const [deleteTarget, setDeleteTarget] = useState<MeterRecord | null>(null)

  const rows = meters.data ?? []
  const allNetworks = networks.data ?? []
  const allTypes = meterTypes.data ?? []
  // Resolve a meter's relations (stored by record id) to display labels.
  const networkLabel = (id?: string) =>
    allNetworks.find((n) => n.id === id)?.content.name ?? '—'
  const typeLabel = (id?: string) =>
    allTypes.find((t) => t.id === id)?.content.name ?? '—'

  return (
    <div className='space-y-4'>
      <div className='flex items-center justify-between'>
        <div>
          <h2 className='text-xl font-semibold'>Meters</h2>
          <p className='text-muted-foreground text-sm'>
            Devices stamped from a meter-type onto a network. Online status and
            last-seen are written by the polling service (read-only here). Add
            meters with the meters wizard.
          </p>
        </div>
      </div>

      <Card className='overflow-x-auto p-0'>
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Key</TableHead>
              <TableHead>Name</TableHead>
              <TableHead>Network</TableHead>
              <TableHead>Type</TableHead>
              <TableHead className='text-center'>Ver</TableHead>
              <TableHead className='text-center'>Addr</TableHead>
              <TableHead>Status</TableHead>
              <TableHead className='text-right'>Actions</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {meters.isLoading ? (
              <TableRow>
                <TableCell colSpan={8} className='text-muted-foreground'>
                  Loading…
                </TableCell>
              </TableRow>
            ) : rows.length === 0 ? (
              <TableRow>
                <TableCell colSpan={8} className='text-muted-foreground'>
                  No meters yet.
                </TableCell>
              </TableRow>
            ) : (
              rows.map((m) => (
                <TableRow key={m.id}>
                  <TableCell className='font-mono text-xs'>
                    {m.content.key}
                  </TableCell>
                  <TableCell>{m.content.name}</TableCell>
                  <TableCell>{networkLabel(m.content.network)}</TableCell>
                  <TableCell>{typeLabel(m.content.meter_type)}</TableCell>
                  <TableCell className='text-center'>
                    {m.content.meter_type_version}
                  </TableCell>
                  <TableCell className='text-center'>
                    {m.content.address}
                  </TableCell>
                  <TableCell>
                    <StatusBadge
                      status={m.content.status}
                      lastSeen={m.content.last_seen}
                    />
                  </TableCell>
                  <TableCell className='text-right'>
                    <Button
                      variant='ghost'
                      size='icon'
                      title='Delete'
                      onClick={() => setDeleteTarget(m)}
                    >
                      <Trash2 className='size-4' />
                    </Button>
                  </TableCell>
                </TableRow>
              ))
            )}
          </TableBody>
        </Table>
      </Card>

      {deleteTarget ? (
        <ConfirmDialog
          open
          onOpenChange={(o) => !o && setDeleteTarget(null)}
          destructive
          title={`Delete ${deleteTarget.content.name}?`}
          desc='This removes the meter. Its registers are not cascaded.'
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
