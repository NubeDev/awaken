/**
 * Data Console (admin) — a self-contained tool for an external engineer to view
 * ALL of a meter's time-series readings and export them to PDF, built for
 * debugging power spikes / energy use. Pick a meter, tick the registers (series)
 * to inspect, choose a trailing window, and read: a multi-series trend chart,
 * per-series debug stats (min/max/avg/last + the largest spike), and the raw
 * pivoted readings table.
 *
 * Data path: reuses the dashboards' readings hooks (features/dashboards/query/
 * batch) — the same windowed, throttled `GET /readings?series&from&to` fan-out —
 * and the shared chart helpers, so this tool tracks the app theme and never opens
 * a second, divergent fetch path. The window anchor is bucketed to the minute and
 * frozen until "Refresh" so the report is a stable snapshot and the readings query
 * keys don't churn (batch.ts NOW_BUCKET_MS rationale).
 *
 * PDF export is print-based (no new dependency): an injected print stylesheet hides
 * the app chrome and the on-screen controls, leaving only `#data-console-report`
 * for the browser's "Save as PDF". recharts draws SVG, so the chart prints crisp.
 */
import { useMemo, useState } from 'react'
import {
  CartesianGrid,
  Line,
  LineChart,
  Legend,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from 'recharts'
import { Download, RefreshCw } from 'lucide-react'
import {
  useGateways,
  useMeters,
  useNetworks,
  useRegisters,
  useRegistersHistory,
  useSites,
} from '@/features/dashboards/query/batch'
import { WINDOW_TOKENS, type WindowToken } from '@/features/dashboards/query/time-window'
import {
  AXIS_LINE,
  AXIS_TICK,
  CHART_MARGIN,
  GRID_PROPS,
} from '@/features/dashboards/_shared/chart-axes'
import { formatTick, formatValue } from '@/features/dashboards/_shared/format-value'
import { seriesColor } from '@/features/dashboards/_shared/palette'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card } from '@/components/ui/card'
import { Checkbox } from '@/components/ui/checkbox'
import { Label } from '@/components/ui/label'
import { ScrollArea } from '@/components/ui/scroll-area'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import { computeStats } from './stats'

const WINDOW_LABELS: Record<WindowToken, string> = {
  'now-6h': 'Last 6 hours',
  'now-24h': 'Last 24 hours',
  'now-7d': 'Last 7 days',
}

/** Cap rows rendered in the raw table (and printed) so a dense series can't blow
 *  up the DOM / PDF. The chart and stats always use the full window. */
const MAX_TABLE_ROWS = 1500

const fmtInstant = (iso: string | null) =>
  iso ? new Date(iso).toLocaleString() : '—'

export function DataConsole() {
  const meters = useMeters()
  const registers = useRegisters()
  const sites = useSites()
  const gateways = useGateways()
  const networks = useNetworks()

  const [meterId, setMeterId] = useState('')
  const [selected, setSelected] = useState<string[]>([]) // register record ids
  const [token, setToken] = useState<WindowToken>('now-24h')
  // Frozen, minute-bucketed window anchor — a stable snapshot; "Refresh" re-anchors.
  const [anchor, setAnchor] = useState(
    () => Math.floor(Date.now() / 60_000) * 60_000
  )

  // Resolve a meter → its site name (network → gateway → site) for the picker label.
  const siteNameForMeter = useMemo(() => {
    const netById = new Map((networks.data ?? []).map((n) => [n.id, n.content]))
    const gwById = new Map((gateways.data ?? []).map((g) => [g.id, g.content]))
    const siteById = new Map((sites.data ?? []).map((s) => [s.id, s.content]))
    return (meterContent: { network?: string }) => {
      const net = meterContent.network ? netById.get(meterContent.network) : undefined
      const gw = net?.gateway ? gwById.get(net.gateway) : undefined
      const site = gw?.site ? siteById.get(gw.site) : undefined
      return site?.name ?? '—'
    }
  }, [networks.data, gateways.data, sites.data])

  const allMeters = meters.data ?? []
  const meter = allMeters.find((m) => m.id === meterId)

  // Registers of the chosen meter that carry history (the ones with readings).
  const meterRegisters = useMemo(
    () =>
      (registers.data ?? [])
        .filter((r) => r.content.meter === meterId && r.content.history !== false)
        .sort((a, b) => a.content.name.localeCompare(b.content.name)),
    [registers.data, meterId]
  )

  const selectedRegisters = meterRegisters.filter((r) => selected.includes(r.id))

  const range = useMemo(() => {
    const hours = WINDOW_TOKENS[token]
    return {
      fromIso: new Date(anchor - hours * 3600_000).toISOString(),
      toIso: new Date(anchor).toISOString(),
    }
  }, [anchor, token])

  const history = useRegistersHistory(
    selectedRegisters.map((r) => ({ id: r.id })),
    { from: range.fromIso, to: range.toIso }
  )

  // Per-series samples, sorted ascending, joined by sample.series === register.id.
  const series = useMemo(
    () =>
      selectedRegisters.map((reg, i) => {
        const samples = history.data
          .filter((s) => s.series === reg.id)
          .sort((a, b) => Date.parse(a.at) - Date.parse(b.at))
        return {
          reg,
          label: reg.content.name,
          unit: reg.content.unit,
          precision: reg.content.precision,
          color: seriesColor(i),
          samples,
          stats: computeStats(samples),
        }
      }),
    [selectedRegisters, history.data]
  )

  // Pivot to one row per instant: { t, [label]: value } — drives chart AND table.
  const rows = useMemo(() => {
    const byT = new Map<number, Record<string, number | null> & { t: number }>()
    for (const s of series) {
      for (const sample of s.samples) {
        const t = Date.parse(sample.at)
        let row = byT.get(t)
        if (!row) {
          row = { t }
          byT.set(t, row)
        }
        row[s.label] = sample.value
      }
    }
    return [...byT.values()].sort((a, b) => a.t - b.t)
  }, [series])

  const tableRows = rows.slice(-MAX_TABLE_ROWS)
  const truncated = rows.length - tableRows.length

  const toggle = (id: string) =>
    setSelected((cur) =>
      cur.includes(id) ? cur.filter((x) => x !== id) : [...cur, id]
    )

  const pickMeter = (id: string) => {
    setMeterId(id)
    setSelected([]) // registers belong to the meter; clear the old selection
  }

  const exportPdf = () => {
    const prev = document.title
    const stamp = new Date().toISOString().slice(0, 10)
    document.title = meter
      ? `meter-${meter.content.key}-data-${stamp}`
      : 'meter-data'
    window.print()
    document.title = prev
  }

  const canReport = meter && series.length > 0

  return (
    <div className='space-y-4'>
      {/* Print stylesheet: hide app chrome + controls, print only the report. */}
      <style>{`
        @media print {
          body * { visibility: hidden !important; }
          #data-console-report, #data-console-report * { visibility: visible !important; }
          #data-console-report { position: absolute !important; left: 0; top: 0; width: 100%; }
          .report-avoid-break { break-inside: avoid; }
          @page { margin: 14mm; }
        }
      `}</style>

      <div className='flex items-start justify-between gap-4 print:hidden'>
        <div>
          <h2 className='text-xl font-semibold'>Data Console</h2>
          <p className='text-muted-foreground text-sm'>
            Browse a meter's raw time-series readings and export a PDF report —
            built for debugging power spikes and energy use.
          </p>
        </div>
        <Button onClick={exportPdf} disabled={!canReport}>
          <Download className='mr-1 size-4' /> Export PDF
        </Button>
      </div>

      {/* Controls — never printed. */}
      <Card className='space-y-4 p-4 print:hidden'>
        <div className='grid gap-4 sm:grid-cols-2'>
          <div className='grid gap-1'>
            <Label htmlFor='dc-meter'>Meter</Label>
            <Select value={meterId} onValueChange={pickMeter}>
              <SelectTrigger id='dc-meter'>
                <SelectValue placeholder='Select a meter' />
              </SelectTrigger>
              <SelectContent>
                {allMeters.map((m) => (
                  <SelectItem key={m.id} value={m.id}>
                    {siteNameForMeter(m.content)} — {m.content.name} ({m.content.key})
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
          <div className='grid gap-1'>
            <Label htmlFor='dc-window'>Window</Label>
            <div className='flex gap-2'>
              <Select value={token} onValueChange={(v) => setToken(v as WindowToken)}>
                <SelectTrigger id='dc-window' className='flex-1'>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {(Object.keys(WINDOW_LABELS) as WindowToken[]).map((t) => (
                    <SelectItem key={t} value={t}>
                      {WINDOW_LABELS[t]}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
              <Button
                variant='outline'
                size='icon'
                title='Refresh window to now'
                onClick={() => setAnchor(Math.floor(Date.now() / 60_000) * 60_000)}
              >
                <RefreshCw className='size-4' />
              </Button>
            </div>
          </div>
        </div>

        {meterId ? (
          <div className='grid gap-2'>
            <Label>Registers (series)</Label>
            {meterRegisters.length === 0 ? (
              <p className='text-muted-foreground text-sm'>
                This meter has no history-bearing registers.
              </p>
            ) : (
              <div className='grid gap-2 sm:grid-cols-2 lg:grid-cols-3'>
                {meterRegisters.map((r) => (
                  <label
                    key={r.id}
                    className='hover:bg-muted/50 flex cursor-pointer items-center gap-2 rounded-md border p-2 text-sm'
                  >
                    <Checkbox
                      checked={selected.includes(r.id)}
                      onCheckedChange={() => toggle(r.id)}
                    />
                    <span className='flex-1'>{r.content.name}</span>
                    <Badge variant='outline' className='text-xs'>
                      {r.content.unit || r.content.quantity || '—'}
                    </Badge>
                  </label>
                ))}
              </div>
            )}
          </div>
        ) : null}
      </Card>

      {/* The report — the only thing that prints. */}
      {canReport ? (
        <div id='data-console-report' className='space-y-4'>
          <div className='report-avoid-break space-y-1'>
            <h1 className='text-2xl font-semibold'>Meter Data Report</h1>
            <div className='text-muted-foreground grid gap-x-8 gap-y-0.5 text-sm sm:grid-cols-2'>
              <span>
                <strong>Meter:</strong> {meter!.content.name} ({meter!.content.key})
              </span>
              <span>
                <strong>Site:</strong> {siteNameForMeter(meter!.content)}
              </span>
              <span>
                <strong>Window:</strong> {WINDOW_LABELS[token]}
              </span>
              <span>
                <strong>Series:</strong> {series.length}
              </span>
              <span>
                <strong>From:</strong> {new Date(range.fromIso).toLocaleString()}
              </span>
              <span>
                <strong>To:</strong> {new Date(range.toIso).toLocaleString()}
              </span>
              <span>
                <strong>Generated:</strong> {new Date().toLocaleString()}
              </span>
            </div>
          </div>

          {/* Per-series debug stats. */}
          <div className='grid gap-3 sm:grid-cols-2 lg:grid-cols-3'>
            {series.map((s) => (
              <Card key={s.reg.id} className='report-avoid-break space-y-2 p-3'>
                <div className='flex items-center gap-2'>
                  <span
                    className='size-3 shrink-0 rounded-full'
                    style={{ backgroundColor: s.color }}
                  />
                  <span className='font-medium'>{s.label}</span>
                  <Badge variant='outline' className='ml-auto text-xs'>
                    {s.unit || '—'}
                  </Badge>
                </div>
                <dl className='grid grid-cols-2 gap-x-4 gap-y-1 text-sm'>
                  <Stat label='Samples' value={String(s.stats.count)} />
                  <Stat
                    label='Last'
                    value={formatValue(s.stats.last, { precision: s.precision, unit: s.unit })}
                  />
                  <Stat
                    label='Min'
                    value={formatValue(s.stats.min, { precision: s.precision, unit: s.unit })}
                  />
                  <Stat
                    label='Max'
                    value={formatValue(s.stats.max, { precision: s.precision, unit: s.unit })}
                  />
                  <Stat
                    label='Average'
                    value={formatValue(s.stats.avg, { precision: s.precision, unit: s.unit })}
                  />
                  <Stat
                    label='Peak spike'
                    value={formatValue(s.stats.peakJump, { precision: s.precision, unit: s.unit })}
                    hint={s.stats.peakJumpAt ? fmtInstant(s.stats.peakJumpAt) : undefined}
                  />
                </dl>
              </Card>
            ))}
          </div>

          {/* Trend chart. */}
          <Card className='report-avoid-break p-3'>
            {history.isLoading ? (
              <p className='text-muted-foreground py-12 text-center text-sm'>
                Loading readings…
              </p>
            ) : rows.length === 0 ? (
              <p className='text-muted-foreground py-12 text-center text-sm'>
                No readings in this window.
              </p>
            ) : (
              <ResponsiveContainer width='100%' height={300}>
                <LineChart data={rows} margin={CHART_MARGIN}>
                  <CartesianGrid {...GRID_PROPS} />
                  <XAxis
                    dataKey='t'
                    type='number'
                    scale='time'
                    domain={['dataMin', 'dataMax']}
                    tickFormatter={(t: number) => new Date(t).toLocaleString()}
                    tick={AXIS_TICK}
                    axisLine={AXIS_LINE}
                    tickLine={false}
                    minTickGap={48}
                  />
                  <YAxis
                    tickFormatter={formatTick}
                    tick={AXIS_TICK}
                    axisLine={false}
                    tickLine={false}
                    width={48}
                  />
                  <Tooltip
                    labelFormatter={(t) => new Date(Number(t)).toLocaleString()}
                    contentStyle={{ fontSize: 12 }}
                  />
                  {series.length > 1 ? (
                    <Legend wrapperStyle={{ fontSize: 11, paddingTop: 4 }} />
                  ) : null}
                  {series.map((s) => (
                    <Line
                      key={s.label}
                      type='monotone'
                      dataKey={s.label}
                      stroke={s.color}
                      strokeWidth={2}
                      dot={false}
                      isAnimationActive={false}
                      connectNulls
                    />
                  ))}
                </LineChart>
              </ResponsiveContainer>
            )}
          </Card>

          {/* Raw readings, pivoted by instant. */}
          <Card className='overflow-hidden p-0'>
            <div className='flex items-center justify-between border-b px-3 py-2 text-sm'>
              <span className='font-medium'>Raw readings</span>
              <span className='text-muted-foreground'>
                {rows.length} rows
                {truncated > 0 ? ` (showing latest ${tableRows.length})` : ''}
              </span>
            </div>
            <ScrollArea className='max-h-[480px] print:max-h-none'>
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Time</TableHead>
                    {series.map((s) => (
                      <TableHead key={s.label} className='text-right'>
                        {s.label}
                        {s.unit ? ` (${s.unit})` : ''}
                      </TableHead>
                    ))}
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {tableRows.map((row) => (
                    <TableRow key={row.t}>
                      <TableCell className='whitespace-nowrap font-mono text-xs'>
                        {new Date(row.t).toLocaleString()}
                      </TableCell>
                      {series.map((s) => (
                        <TableCell key={s.label} className='text-right font-mono text-xs'>
                          {typeof row[s.label] === 'number'
                            ? (row[s.label] as number).toFixed(s.precision ?? 2)
                            : '—'}
                        </TableCell>
                      ))}
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </ScrollArea>
          </Card>
        </div>
      ) : (
        <Card className='text-muted-foreground p-8 text-center text-sm print:hidden'>
          Pick a meter and at least one register to build a report.
        </Card>
      )}
    </div>
  )
}

function Stat({
  label,
  value,
  hint,
}: {
  label: string
  value: string
  hint?: string
}) {
  return (
    <div className='flex flex-col'>
      <dt className='text-muted-foreground text-xs'>{label}</dt>
      <dd className='font-medium'>{value}</dd>
      {hint ? <dd className='text-muted-foreground text-xs'>{hint}</dd> : null}
    </div>
  )
}
