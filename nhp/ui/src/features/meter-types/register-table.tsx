/**
 * The register-map editor table (ADMIN.md §1, DOMAIN-MODEL §register): a row per
 * register over the meter-type's `registers[]`, every field editable, enum fields
 * as dropdowns, an inline alarm ramp, add-row, and a bulk CSV/JSON import. Holds
 * the working register array and reports changes upward; the parent edit form owns
 * the save (which bumps the version).
 */
import { useState } from 'react'
import { ClipboardPaste, Plus } from 'lucide-react'
import type { RegisterDef } from '@/api/records'
import { Button } from '@/components/ui/button'
import {
  Table,
  TableBody,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import { BulkImportDialog } from './bulk-import-dialog'
import { blankRegister } from './register-defaults'
import { RegisterRow } from './register-row'

type RegisterTableProps = {
  registers: RegisterDef[]
  onChange: (registers: RegisterDef[]) => void
}

const COLUMNS = [
  'Key',
  'Name',
  'Addr',
  'Fn code',
  'Datatype',
  'Words',
  'Byte order',
  'Scale',
  'Offset',
  'Signed',
  'Unit',
  'Quantity',
  'History',
  'Chart',
  'Group',
  'Prec.',
  '',
]

export function RegisterTable({ registers, onChange }: RegisterTableProps) {
  const [bulkOpen, setBulkOpen] = useState(false)

  const updateAt = (i: number, reg: RegisterDef) =>
    onChange(registers.map((r, idx) => (idx === i ? reg : r)))
  const removeAt = (i: number) =>
    onChange(registers.filter((_, idx) => idx !== i))
  const addRow = () =>
    onChange([...registers, blankRegister(registers.length)])
  const appendImported = (defs: RegisterDef[]) =>
    onChange([...registers, ...defs])

  return (
    <div className='space-y-3'>
      <div className='flex items-center justify-between'>
        <p className='text-muted-foreground text-sm'>
          {registers.length} register{registers.length === 1 ? '' : 's'}
        </p>
        <div className='flex gap-2'>
          <Button type='button' variant='outline' size='sm' onClick={() => setBulkOpen(true)}>
            <ClipboardPaste className='mr-1 size-4' /> Bulk import
          </Button>
          <Button type='button' variant='outline' size='sm' onClick={addRow}>
            <Plus className='mr-1 size-4' /> Add register
          </Button>
        </div>
      </div>

      <div className='overflow-x-auto rounded-md border'>
        <Table>
          <TableHeader>
            <TableRow>
              {COLUMNS.map((c, i) => (
                <TableHead key={i} className='whitespace-nowrap text-xs'>
                  {c}
                </TableHead>
              ))}
            </TableRow>
          </TableHeader>
          <TableBody>
            {registers.map((reg, i) => (
              <RegisterRow
                key={i}
                reg={reg}
                onChange={(r) => updateAt(i, r)}
                onRemove={() => removeAt(i)}
              />
            ))}
          </TableBody>
        </Table>
      </div>

      <BulkImportDialog
        open={bulkOpen}
        onOpenChange={setBulkOpen}
        startIndex={registers.length}
        onImport={appendImported}
      />
    </div>
  )
}
