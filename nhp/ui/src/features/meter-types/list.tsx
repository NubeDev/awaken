/**
 * Meter-types admin landing (ADMIN.md §1): a table of every `kind:"meter-type"`
 * with its version, register count, and deployed-meter rollup (how many meters
 * are on an older version — DOMAIN-MODEL §versioning). Row actions: edit, clone,
 * re-apply to a meter, delete. Selecting create/edit/clone swaps to the full-page
 * editor. All reads/writes go through the rubix records API.
 */
import { useState } from 'react'
import { Copy, Pencil, Plus, QrCode, RefreshCw, Trash2 } from 'lucide-react'
import type { MeterTypeRecord } from '@/api/records'
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
import { BarcodeLabel } from './barcode-label'
import { MeterTypeEditor, type EditMode } from './edit'
import {
  useDeleteMeterType,
  useMeters,
  useMeterTypes,
  useRegisters,
} from './hooks'
import { ReapplyDialog } from './reapply-dialog'
import { rollupForType } from './versioning'

export function MeterTypeList() {
  const types = useMeterTypes()
  const meters = useMeters()
  const registers = useRegisters()
  const del = useDeleteMeterType()

  const [editing, setEditing] = useState<EditMode | null>(null)
  const [deleteTarget, setDeleteTarget] = useState<MeterTypeRecord | null>(null)
  const [reapplyTarget, setReapplyTarget] = useState<MeterTypeRecord | null>(
    null
  )
  const [barcodeTarget, setBarcodeTarget] = useState<MeterTypeRecord | null>(
    null
  )

  if (editing) {
    return (
      <MeterTypeEditor state={editing} onDone={() => setEditing(null)} />
    )
  }

  const rows = types.data ?? []
  const allMeters = meters.data ?? []

  return (
    <div className='space-y-4'>
      <div className='flex items-center justify-between'>
        <div>
          <h2 className='text-xl font-semibold'>Meter-types</h2>
          <p className='text-muted-foreground text-sm'>
            Templates a meter is stamped from — the register map, units, history,
            charts and alarms.
          </p>
        </div>
        <Button onClick={() => setEditing({ mode: 'create' })}>
          <Plus className='mr-1 size-4' /> New meter-type
        </Button>
      </div>

      <Card className='overflow-x-auto p-0'>
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Key</TableHead>
              <TableHead>Name</TableHead>
              <TableHead>Manufacturer</TableHead>
              <TableHead className='text-center'>Version</TableHead>
              <TableHead className='text-center'>Registers</TableHead>
              <TableHead className='text-center'>Deployed</TableHead>
              <TableHead className='text-right'>Actions</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {types.isLoading ? (
              <TableRow>
                <TableCell colSpan={7} className='text-muted-foreground'>
                  Loading…
                </TableCell>
              </TableRow>
            ) : rows.length === 0 ? (
              <TableRow>
                <TableCell colSpan={7} className='text-muted-foreground'>
                  No meter-types yet.
                </TableCell>
              </TableRow>
            ) : (
              rows.map((t) => {
                const roll = rollupForType(
                  t.id,
                  t.content.version,
                  allMeters
                )
                return (
                  <TableRow key={t.id}>
                    <TableCell className='font-mono text-xs'>
                      {t.content.key}
                    </TableCell>
                    <TableCell>{t.content.name}</TableCell>
                    <TableCell>{t.content.manufacturer ?? '—'}</TableCell>
                    <TableCell className='text-center'>
                      v{t.content.version}
                    </TableCell>
                    <TableCell className='text-center'>
                      {t.content.registers?.length ?? 0}
                    </TableCell>
                    <TableCell className='text-center'>
                      {roll.total === 0 ? (
                        <span className='text-muted-foreground'>0</span>
                      ) : (
                        <span className='inline-flex items-center gap-1'>
                          {roll.total}
                          {roll.outOfDate > 0 ? (
                            <Badge variant='outline' className='text-amber-600'>
                              {roll.outOfDate} out of date
                            </Badge>
                          ) : null}
                        </span>
                      )}
                    </TableCell>
                    <TableCell className='text-right'>
                      <div className='flex justify-end gap-1'>
                        <Button
                          variant='ghost'
                          size='icon'
                          title='Edit'
                          onClick={() =>
                            setEditing({ mode: 'edit', record: t })
                          }
                        >
                          <Pencil className='size-4' />
                        </Button>
                        <Button
                          variant='ghost'
                          size='icon'
                          title='Clone as new'
                          onClick={() =>
                            setEditing({ mode: 'clone', source: t })
                          }
                        >
                          <Copy className='size-4' />
                        </Button>
                        <Button
                          variant='ghost'
                          size='icon'
                          title='Show / print scan barcode'
                          onClick={() => setBarcodeTarget(t)}
                        >
                          <QrCode className='size-4' />
                        </Button>
                        <Button
                          variant='ghost'
                          size='icon'
                          title='Re-apply to a meter'
                          disabled={roll.total === 0}
                          onClick={() => setReapplyTarget(t)}
                        >
                          <RefreshCw className='size-4' />
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

      {deleteTarget ? (
        <ConfirmDialog
          open={true}
          onOpenChange={(o) => !o && setDeleteTarget(null)}
          destructive
          title={`Delete ${deleteTarget.content.name}?`}
          desc={
            <>
              This removes the meter-type template. Meters already stamped from it
              keep their own registers and are not affected.
            </>
          }
          confirmText='Delete'
          isLoading={del.isPending}
          handleConfirm={() =>
            del.mutate(deleteTarget.id, {
              onSuccess: () => setDeleteTarget(null),
            })
          }
        />
      ) : null}

      {barcodeTarget ? (
        <BarcodeLabel
          type={barcodeTarget}
          open={true}
          onOpenChange={(o) => !o && setBarcodeTarget(null)}
        />
      ) : null}

      {reapplyTarget ? (
        <ReapplyDialog
          open={true}
          onOpenChange={(o) => !o && setReapplyTarget(null)}
          type={reapplyTarget}
          meters={allMeters}
          registers={registers.data ?? []}
        />
      ) : null}
    </div>
  )
}
