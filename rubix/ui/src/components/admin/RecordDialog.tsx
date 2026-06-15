// Create/edit a record by editing its raw `content` JSON. The store is genuinely
// schemaless, so the escape hatch IS the editor — every record is editable as the
// JSON it is, with no per-domain form (ADMIN-UI: <JsonContentField> "never blocks
// an unknown shape"). Validation here is only that the text parses as a JSON
// object; the gate enforces any registered collection schema server-side.

import { useEffect, useState } from 'react'
import { Button } from '../ui/button'
import { Textarea } from '../ui/textarea'
import { Label } from '../ui/label'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '../ui/dialog'
import type { Record, RecordContent } from '../../types/Record'

interface RecordDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  /** The record being edited; omit to create a new one. */
  record?: Record
  onSubmit: (content: RecordContent) => Promise<void>
  saving: boolean
}

export function RecordDialog({ open, onOpenChange, record, onSubmit, saving }: RecordDialogProps) {
  const [text, setText] = useState('')
  const [error, setError] = useState<string | null>(null)

  // Reset the editor whenever the target record (or create-mode) changes.
  useEffect(() => {
    if (!open) return
    setError(null)
    setText(JSON.stringify(record?.content ?? { kind: '' }, null, 2))
  }, [open, record])

  async function handleSave() {
    let parsed: unknown
    try {
      parsed = JSON.parse(text)
    } catch (e) {
      setError(`Invalid JSON: ${e instanceof Error ? e.message : String(e)}`)
      return
    }
    if (typeof parsed !== 'object' || parsed === null || Array.isArray(parsed)) {
      setError('Content must be a JSON object.')
      return
    }
    setError(null)
    await onSubmit(parsed as RecordContent)
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-2xl">
        <DialogHeader>
          <DialogTitle>{record ? 'Edit record' : 'New record'}</DialogTitle>
          <DialogDescription>
            {record ? (
              <span className="mono text-xs">{record.id}</span>
            ) : (
              'Content is free-form JSON. The `kind` field is the only convention.'
            )}
          </DialogDescription>
        </DialogHeader>
        <div className="flex flex-col gap-2">
          <Label htmlFor="record-content">Content</Label>
          <Textarea
            id="record-content"
            value={text}
            onChange={(e) => setText(e.target.value)}
            spellCheck={false}
            className="mono min-h-[280px] text-xs leading-relaxed"
          />
          {error && <p className="text-xs text-destructive">{error}</p>}
        </div>
        <DialogFooter>
          <Button variant="ghost" onClick={() => onOpenChange(false)} disabled={saving}>
            Cancel
          </Button>
          <Button onClick={handleSave} disabled={saving}>
            {saving ? 'Saving…' : record ? 'Save changes' : 'Create record'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
