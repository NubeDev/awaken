/**
 * The bulk-paste import dialog: an admin pastes a CSV or JSON register table and
 * it is appended to the meter-type's register set (ADMIN.md §1). Parsing lives in
 * bulk-import.ts; this is the surface.
 */
import { useState } from 'react'
import type { RegisterDef } from '@/api/records'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Textarea } from '@/components/ui/textarea'
import { parseRegisterTable } from './bulk-import'

type BulkImportDialogProps = {
  open: boolean
  onOpenChange: (open: boolean) => void
  /** Index the appended rows start at (for default keys/names). */
  startIndex: number
  onImport: (defs: RegisterDef[]) => void
}

export function BulkImportDialog({
  open,
  onOpenChange,
  startIndex,
  onImport,
}: BulkImportDialogProps) {
  const [text, setText] = useState('')
  const [error, setError] = useState<string | null>(null)

  const submit = () => {
    try {
      const defs = parseRegisterTable(text, startIndex)
      onImport(defs)
      setText('')
      setError(null)
      onOpenChange(false)
    } catch (e) {
      setError((e as Error).message)
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className='sm:max-w-2xl'>
        <DialogHeader>
          <DialogTitle>Bulk import registers</DialogTitle>
          <DialogDescription>
            Paste a JSON array of register objects, or CSV with a header row of
            field names (key, name, address, fn_code, datatype, …). Rows are
            appended to the current register set.
          </DialogDescription>
        </DialogHeader>
        <Textarea
          className='min-h-48 font-mono text-xs'
          placeholder={
            'key,name,address,datatype,unit,quantity,history\nvoltage_l1,Voltage L1,3027,float32,V,voltage,true'
          }
          value={text}
          onChange={(e) => setText(e.target.value)}
        />
        {error ? <p className='text-destructive text-sm'>{error}</p> : null}
        <DialogFooter>
          <Button variant='ghost' onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button onClick={submit}>Import</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
