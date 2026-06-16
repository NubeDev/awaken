/**
 * The networks belonging to one gateway (ADMIN.md §5). A compact table: key, type,
 * protocol, the net-type params summary, and a device-count/cap badge
 * (capacity-badge.tsx). Add / edit (network-form.tsx) and delete. Networks per
 * gateway are unlimited (DOMAIN-MODEL §gateway).
 */
import { useState } from 'react'
import { Pencil, Plus, Trash2 } from 'lucide-react'
import type {
  GatewayRecord,
  MeterRecord,
  Net485Params,
  NetEthernetParams,
  NetworkRecord,
} from '@/api/records'
import { Button } from '@/components/ui/button'
import { ConfirmDialog } from '@/components/confirm-dialog'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import { CapacityBadge } from './capacity-badge'
import { useDeleteNetwork } from './hooks'
import { NetworkForm } from './network-form'

/** One-line summary of a network's net-type params for the table. */
function paramsSummary(net: NetworkRecord): string {
  const p = net.content.params
  if (!p) return '—'
  if (net.content.net_type === '485') {
    const s = p as Net485Params
    return `${s.baud} ${s.data_bits}${s.parity[0].toUpperCase()}${s.stop_bits}`
  }
  const e = p as NetEthernetParams
  return `${e.ip}:${e.port}`
}

export function NetworkTable({
  gateway,
  networks,
  meters,
}: {
  gateway: GatewayRecord
  networks: NetworkRecord[]
  meters: MeterRecord[]
}) {
  const del = useDeleteNetwork()
  const [adding, setAdding] = useState(false)
  const [editing, setEditing] = useState<NetworkRecord | null>(null)
  const [deleteTarget, setDeleteTarget] = useState<NetworkRecord | null>(null)

  return (
    <div className='space-y-2'>
      <div className='flex items-center justify-between'>
        <h4 className='text-sm font-medium'>Networks</h4>
        <Button size='sm' variant='outline' onClick={() => setAdding(true)}>
          <Plus className='mr-1 size-4' /> Add network
        </Button>
      </div>

      {networks.length === 0 ? (
        <p className='text-muted-foreground text-sm'>No networks on this gateway.</p>
      ) : (
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Key</TableHead>
              <TableHead>Type</TableHead>
              <TableHead>Protocol</TableHead>
              <TableHead>Params</TableHead>
              <TableHead className='text-center'>Devices</TableHead>
              <TableHead className='text-right'>Actions</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {networks.map((n) => (
              <TableRow key={n.id}>
                <TableCell className='font-mono text-xs'>
                  {n.content.key}
                </TableCell>
                <TableCell>{n.content.net_type}</TableCell>
                <TableCell>{n.content.protocol}</TableCell>
                <TableCell className='font-mono text-xs'>
                  {paramsSummary(n)}
                </TableCell>
                <TableCell className='text-center'>
                  <CapacityBadge network={n} meters={meters} />
                </TableCell>
                <TableCell className='text-right'>
                  <div className='flex justify-end gap-1'>
                    <Button
                      variant='ghost'
                      size='icon'
                      title='Edit'
                      onClick={() => setEditing(n)}
                    >
                      <Pencil className='size-4' />
                    </Button>
                    <Button
                      variant='ghost'
                      size='icon'
                      title='Delete'
                      onClick={() => setDeleteTarget(n)}
                    >
                      <Trash2 className='size-4' />
                    </Button>
                  </div>
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      )}

      {adding ? (
        <NetworkForm
          open
          onOpenChange={(o) => !o && setAdding(false)}
          gateway={gateway}
          meters={meters}
        />
      ) : null}

      {editing ? (
        <NetworkForm
          open
          onOpenChange={(o) => !o && setEditing(null)}
          gateway={gateway}
          network={editing}
          meters={meters}
        />
      ) : null}

      {deleteTarget ? (
        <ConfirmDialog
          open
          onOpenChange={(o) => !o && setDeleteTarget(null)}
          destructive
          title={`Delete network ${deleteTarget.content.key}?`}
          desc='Meters on this network are not removed but will be orphaned. Remove or move them first.'
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
