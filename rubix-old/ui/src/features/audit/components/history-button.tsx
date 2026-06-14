/**
 * A toolbar button that opens a resource's History tab in a dialog
 * (docs/design/audit-and-undo.md "UI": a per-resource History tab on a dashboard,
 * datasource, rule, …). Wrapping the `HistoryTab` in a dialog lets any resource
 * detail view surface its change history with one button, without restructuring
 * the view into tabs.
 */
import { useState } from 'react'
import { History } from 'lucide-react'
import type { Uuid } from '@/api/types'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { HistoryTab } from './history-tab'

export function HistoryButton({
  kind,
  id,
  label = 'History',
  resourceName,
}: {
  kind: string
  id: Uuid
  label?: string
  /** Optional friendly name for the dialog title. */
  resourceName?: string
}) {
  const [open, setOpen] = useState(false)

  return (
    <>
      <Button size='sm' variant='outline' onClick={() => setOpen(true)}>
        <History className='size-3.5' /> {label}
      </Button>
      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent className='sm:max-w-2xl'>
          <DialogHeader>
            <DialogTitle>
              History{resourceName ? ` · ${resourceName}` : ''}
            </DialogTitle>
          </DialogHeader>
          <div className='scroll max-h-[60vh] overflow-y-auto pe-1'>
            {open ? <HistoryTab kind={kind} id={id} /> : null}
          </div>
        </DialogContent>
      </Dialog>
    </>
  )
}
