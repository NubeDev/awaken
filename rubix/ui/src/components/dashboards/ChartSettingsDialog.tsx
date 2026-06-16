// Edit a dashboard panel's chart in place, with the SAME controls as the Query
// console: an editable SQL field (with Run), the structural ChartConfigBar
// (type/x/y/breakdown/display/quantity), the FieldConfigEditor (axis unit), and the
// TransformEditor (aggregate + cosmetic pipeline) — all reused, not reimplemented.
// A live preview renders the edited config; it seeds from the panel's current batch
// rows but can be re-run against the edited SQL right here (so a brand-new blank
// chart added inline can be authored end-to-end without leaving the board). Saving
// writes the chart record (name + sql + config) via the caller; the board re-batches
// and every panel referencing that chart updates.

import { useMemo, useState } from 'react'
import { useMutation } from '@tanstack/react-query'
import { Play } from 'lucide-react'
import { useApi } from '../../api/ConnectionContext'
import type { SavedChart } from '../../api/charts'
import { runQuery, type QueryColumn, type TimeScope } from '../../api/query'
import { splitTransforms as splitForRun } from '../chart-builder/transforms'
import { Button } from '../ui/button'
import { Input } from '../ui/input'
import { Label } from '../ui/label'
import { Textarea } from '../ui/textarea'
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
  /** Tenant whose scoped api runs the in-dialog "Run" preview. */
  tenant: string
  chart: SavedChart
  /** The panel's current rows (the board's batch result) — seeds the preview before
   *  the operator re-runs the edited SQL. */
  rows: Record<string, unknown>[]
  /** Backend column types when present, preferred over sniffing the rows. */
  columns?: QueryColumn[]
  /** The board's time scope, so an in-dialog Run previews the same window the board
   *  would render. */
  time?: TimeScope
  /** Persist the edited name + sql + config (the caller does the updateChart mutation). */
  onSave: (input: { name: string; sql: string; config: ChartConfig }) => void
  saving?: boolean
}

export function ChartSettingsDialog(props: ChartSettingsDialogProps) {
  return (
    <Dialog open={props.open} onOpenChange={props.onOpenChange}>
      {/* Cap the dialog to the viewport and lay it out as a flex column: the header
          and footer stay pinned while the form body scrolls. Without this a tall
          chart (long query + many transforms) runs off the bottom of a short window
          with the Save/Cancel footer unreachable. */}
      <DialogContent className="flex max-h-[90vh] max-w-3xl flex-col gap-0 overflow-hidden">
        {props.open ? <Body {...props} /> : null}
      </DialogContent>
    </Dialog>
  )
}

function Body({ tenant, chart, rows, columns, time, onOpenChange, onSave, saving }: ChartSettingsDialogProps) {
  const api = useApi(tenant)
  // Edit a working copy; only commit on Save so Cancel discards cleanly.
  const [name, setName] = useState(chart.name)
  const [sql, setSql] = useState(chart.sql)
  const [config, setConfig] = useState<ChartConfig>(chart.config)

  // The rows the preview draws from. Seeded by the board's batch result; replaced
  // when the operator runs the edited SQL here. Likewise its column types: prefer a
  // fresh run's columns, then the board-supplied columns, else sniff the rows.
  const [ranRows, setRanRows] = useState<Record<string, unknown>[] | null>(null)
  const [ranCols, setRanCols] = useState<QueryColumn[] | null>(null)
  const baseRows = ranRows ?? rows

  // Run the edited SQL against the board's scope + time window. Only the aggregate
  // transform tier goes to the backend (the cosmetic tier runs client-side below),
  // matching what the board's batch sends — so the preview is faithful.
  const run = useMutation({
    mutationFn: () =>
      runQuery(api, sql, {
        time,
        quantities: config.quantities,
        transforms: splitForRun(config.transforms).aggregate,
      }),
    onSuccess: (res) => {
      setRanRows(res.rows)
      setRanCols(res.columns)
    },
  })

  // Column list for the pickers: prefer a fresh run's columns, then the board's
  // typed columns, else sniff the rows (same fallback the panel uses).
  const cols: ColumnInfo[] = useMemo(() => {
    const typed = ranCols ?? columns
    if (typed && typed.length > 0) {
      return typed.map((c) => ({ name: c.name, type: coarseType(c.type) }))
    }
    return transformDataToColumns(baseRows as DataRow[])
  }, [ranCols, columns, baseRows])

  // Live preview: the cosmetic transform tier runs client-side (the aggregate tier
  // already ran server-side), exactly as the board does.
  const previewRows = useMemo(() => {
    const cosmetic = splitTransforms(config.transforms).cosmetic
    return cosmetic.length > 0 ? applyCosmeticTransforms(baseRows, cosmetic) : baseRows
  }, [baseRows, config.transforms])

  return (
    <>
      <DialogHeader className="shrink-0 pb-4 pr-8">
        <DialogTitle>Edit chart</DialogTitle>
        <DialogDescription>
          Same controls as the Query console. Changes apply to every dashboard panel
          using this chart.
        </DialogDescription>
      </DialogHeader>

      {/* The scrolling region — grows to fill the capped height, scrolls when the
          content (query + preview + transforms) exceeds it. */}
      <div className="min-h-0 flex-1 space-y-4 overflow-y-auto py-1 pr-1">
        <div className="space-y-1.5">
          <Label className="text-[12px]">Name</Label>
          <Input value={name} onChange={(e) => setName(e.target.value)} placeholder="Chart name" />
        </div>

        <div className="space-y-1.5">
          <div className="flex items-center justify-between">
            <Label className="text-[12px]">Query</Label>
            <Button
              variant="outline"
              size="sm"
              className="h-7 gap-1.5"
              onClick={() => run.mutate()}
              disabled={run.isPending || !sql.trim()}
            >
              <Play size={12} /> {run.isPending ? 'Running…' : 'Run'}
            </Button>
          </div>
          <Textarea
            value={sql}
            onChange={(e) => setSql(e.target.value)}
            spellCheck={false}
            rows={4}
            placeholder="SELECT …"
            className="font-mono text-[12px]"
          />
          {run.error && (
            <div className="text-[12px] text-destructive">{(run.error as Error).message}</div>
          )}
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

      <DialogFooter className="shrink-0 pt-4">
        <Button variant="ghost" onClick={() => onOpenChange(false)}>
          Cancel
        </Button>
        <Button
          onClick={() => onSave({ name: name.trim() || chart.name, sql, config })}
          disabled={saving}
        >
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
