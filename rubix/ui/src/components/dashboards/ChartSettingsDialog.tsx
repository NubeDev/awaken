// Edit a dashboard panel's chart in place, with the SAME controls as the Query
// console: the structural ChartConfigBar (type/x/y/breakdown/display/quantity),
// the FieldConfigEditor (axis unit), and the TransformEditor (aggregate + cosmetic
// pipeline) — all reused, not reimplemented. A live preview renders the edited
// config against the panel's current rows so the operator sees the change before
// saving. Saving writes the chart record (name + config) via the caller; the board
// re-batches and every panel referencing that chart updates.

import { useMemo, useState } from 'react'
import type { SavedChart } from '../../api/charts'
import type { QueryColumn } from '../../api/query'
import { Button } from '../ui/button'
import { Input } from '../ui/input'
import { Label } from '../ui/label'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '../ui/dialog'
import { ChartConfigBar } from '../chart-builder/ChartConfigBar'
import { FieldConfigEditor } from '../chart-builder/FieldConfigEditor'
import { TransformEditor } from '../chart-builder/TransformEditor'
import { ChartRendererCore } from '../chart-builder/charts'
import type { ChartConfig } from '../chart-builder/types'
import type { FieldConfig } from '../chart-builder/field-config'
import type { Transform } from '../chart-builder/transforms'
import { applyCosmeticTransforms, splitTransforms } from '../chart-builder/transforms'
import { transformDataToColumns, type ColumnInfo, type DataRow } from '../chart-builder/utils'

interface ChartSettingsDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  chart: SavedChart
  /** The panel's current rows (the board's batch result) — drives the preview. */
  rows: Record<string, unknown>[]
  /** Backend column types when present, preferred over sniffing the rows. */
  columns?: QueryColumn[]
  /** Persist the edited name + config (the caller does the updateChart mutation). */
  onSave: (input: { name: string; config: ChartConfig }) => void
  saving?: boolean
}

export function ChartSettingsDialog(props: ChartSettingsDialogProps) {
  return (
    <Dialog open={props.open} onOpenChange={props.onOpenChange}>
      <DialogContent className="max-w-3xl">
        {props.open ? <Body {...props} /> : null}
      </DialogContent>
    </Dialog>
  )
}

function Body({ chart, rows, columns, onOpenChange, onSave, saving }: ChartSettingsDialogProps) {
  // Edit a working copy; only commit on Save so Cancel discards cleanly.
  const [name, setName] = useState(chart.name)
  const [config, setConfig] = useState<ChartConfig>(chart.config)

  // Column list for the pickers: prefer the backend's typed columns, else sniff
  // the rows (same fallback the panel uses).
  const cols: ColumnInfo[] = useMemo(() => {
    if (columns && columns.length > 0) {
      return columns.map((c) => ({ name: c.name, type: coarseType(c.type) }))
    }
    return transformDataToColumns(rows as DataRow[])
  }, [columns, rows])

  // Live preview: the cosmetic transform tier runs client-side (the aggregate tier
  // already ran server-side on the board's batch), exactly as the board does.
  const previewRows = useMemo(() => {
    const cosmetic = splitTransforms(config.transforms).cosmetic
    return cosmetic.length > 0 ? applyCosmeticTransforms(rows, cosmetic) : rows
  }, [rows, config.transforms])

  return (
    <>
      <DialogHeader>
        <DialogTitle>Edit chart</DialogTitle>
        <DialogDescription>
          Same controls as the Query console. Changes apply to every dashboard panel
          using this chart.
        </DialogDescription>
      </DialogHeader>

      <div className="space-y-4 py-1">
        <div className="space-y-1.5">
          <Label className="text-[12px]">Name</Label>
          <Input value={name} onChange={(e) => setName(e.target.value)} placeholder="Chart name" />
        </div>

        <div className="flex flex-wrap items-center gap-2">
          <ChartConfigBar columns={cols} config={config} onChange={setConfig} />
          <FieldConfigEditor
            value={config.fieldConfig}
            onChange={(fieldConfig: FieldConfig | undefined) => setConfig({ ...config, fieldConfig })}
          />
        </div>

        <div className="h-[300px] rounded-xl border border-border bg-card/40 p-3">
          {previewRows.length === 0 ? (
            <div className="grid h-full place-items-center text-sm text-muted-foreground">
              No rows to preview.
            </div>
          ) : (
            <ChartRendererCore
              config={config}
              data={previewRows}
              columns={cols}
              syncId="chart-settings"
            />
          )}
        </div>

        <div className="rounded-xl border border-border bg-card/40 p-3">
          <TransformEditor
            value={config.transforms}
            onChange={(transforms: Transform[]) => setConfig({ ...config, transforms })}
          />
        </div>
      </div>

      <DialogFooter>
        <Button variant="ghost" onClick={() => onOpenChange(false)}>
          Cancel
        </Button>
        <Button onClick={() => onSave({ name: name.trim() || chart.name, config })} disabled={saving}>
          {saving ? 'Saving…' : 'Save chart'}
        </Button>
      </DialogFooter>
    </>
  )
}

// Map the backend's coarse column type onto the chart layer's narrower set — same
// mapping ChartPanel uses for its column sniffing.
function coarseType(type: QueryColumn['type']): 'string' | 'number' | 'boolean' {
  if (type === 'number') return 'number'
  if (type === 'boolean') return 'boolean'
  return 'string'
}
