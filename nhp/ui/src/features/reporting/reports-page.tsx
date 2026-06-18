/**
 * Reports page (sidebar → Report → Reports). A scope filter (tenant → site →
 * meter-type → quantity) + a trailing window + a report-type switch drive one of
 * four printable report bodies: energy consumption, multi-meter trends, an alarm
 * summary, or a raw readings export. "Export PDF" prints just the report region
 * (report-chrome PrintStyles). Scope is "all sites for a tenant" by default and
 * drills to a single site — the tenant/site reporting the brief asked for.
 */
import { useState } from 'react'
import { Download } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Card } from '@/components/ui/card'
import { Label } from '@/components/ui/label'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { Main } from '@/components/layout/main'
import { type WindowToken } from '@/features/dashboards/query/time-window'
import { usePortfolio } from './use-portfolio'
import { FilterBar } from './filter-bar'
import { type ScopeFilter } from './scope'
import {
  PrintStyles,
  REPORT_ID,
  printDocument,
  scopeSummary,
} from './report-chrome'
import { ConsumptionReport } from './reports/consumption'
import { TrendReport } from './reports/trend'
import { AlarmSummaryReport } from './reports/alarm-summary'
import { RawReport } from './reports/raw'
import { SiteOverviewReport } from './reports/site-overview'

type ReportType = 'consumption' | 'trend' | 'alarms' | 'raw' | 'site-overview'

const REPORT_TYPES: { value: ReportType; label: string }[] = [
  { value: 'site-overview', label: 'Site overview' },
  { value: 'consumption', label: 'Energy consumption' },
  { value: 'trend', label: 'Trends (charts)' },
  { value: 'alarms', label: 'Alarm summary' },
  { value: 'raw', label: 'Raw readings' },
]

const WINDOW_LABELS: Record<WindowToken, string> = {
  'now-6h': 'Last 6 hours',
  'now-24h': 'Last 24 hours',
  'now-7d': 'Last 7 days',
}

export function ReportsPage() {
  const { index, isLoading } = usePortfolio()
  const [filter, setFilter] = useState<ScopeFilter>({})
  const [token, setToken] = useState<WindowToken>('now-24h')
  const [type, setType] = useState<ReportType>('consumption')

  const labels = scopeSummary(index, filter)
  const typeLabel = REPORT_TYPES.find((t) => t.value === type)!.label
  const windowed = type !== 'alarms'
  // The site overview is a single-site document — Export stays disabled until a
  // site is picked, so a handed-off PDF always names one site.
  const siteRequired = type === 'site-overview'
  const exportBlocked = siteRequired && !filter.siteId

  const exportPdf = () => {
    const slug = (s: string) => s.toLowerCase().replace(/\s+/g, '-')
    const date = new Date().toISOString().slice(0, 10)
    printDocument(
      siteRequired
        ? `site-overview-${slug(labels.site)}-${date}`
        : `report-${type}-${slug(labels.tenant)}-${date}`
    )
  }

  return (
    <Main>
      <PrintStyles />

      <div className='flex items-start justify-between gap-4 print:hidden'>
        <div>
          <h2 className='text-xl font-semibold'>Reports</h2>
          <p className='text-muted-foreground text-sm'>
            Energy, trend, alarm and raw-data reports for a tenant (all its sites)
            or a single site. Export any view to PDF.
          </p>
        </div>
        <Button onClick={exportPdf} disabled={isLoading || exportBlocked}>
          <Download className='mr-1 size-4' /> Export PDF
        </Button>
      </div>

      <Card className='my-4 space-y-4 p-4 print:hidden'>
        <FilterBar index={index} filter={filter} onChange={setFilter} />
        <div className='grid gap-3 sm:grid-cols-2 lg:grid-cols-4'>
          <div className='grid gap-1'>
            <Label className='text-xs'>Report</Label>
            <Select value={type} onValueChange={(v) => setType(v as ReportType)}>
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {REPORT_TYPES.map((t) => (
                  <SelectItem key={t.value} value={t.value}>
                    {t.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
          {windowed ? (
            <div className='grid gap-1'>
              <Label className='text-xs'>Window</Label>
              <Select value={token} onValueChange={(v) => setToken(v as WindowToken)}>
                <SelectTrigger>
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
            </div>
          ) : null}
        </div>
      </Card>

      <div id={REPORT_ID} className='space-y-4'>
        <div className='report-avoid-break space-y-1'>
          <h1 className='text-2xl font-semibold'>{typeLabel} report</h1>
          <div className='text-muted-foreground grid gap-x-8 gap-y-0.5 text-sm sm:grid-cols-2'>
            <span><strong>Tenant:</strong> {labels.tenant}</span>
            <span><strong>Site:</strong> {labels.site}</span>
            <span><strong>Meter-type:</strong> {labels.meterType}</span>
            <span><strong>Quantity:</strong> {labels.quantity}</span>
            {windowed ? (
              <span><strong>Window:</strong> {WINDOW_LABELS[token]}</span>
            ) : null}
            <span><strong>Generated:</strong> {new Date().toLocaleString()}</span>
          </div>
        </div>

        {isLoading ? (
          <Card className='text-muted-foreground p-8 text-center text-sm'>
            Loading portfolio…
          </Card>
        ) : type === 'site-overview' ? (
          <SiteOverviewReport index={index} filter={filter} token={token} />
        ) : type === 'consumption' ? (
          <ConsumptionReport index={index} filter={filter} token={token} />
        ) : type === 'trend' ? (
          <TrendReport index={index} filter={filter} token={token} />
        ) : type === 'alarms' ? (
          <AlarmSummaryReport index={index} filter={filter} />
        ) : (
          <RawReport index={index} filter={filter} token={token} />
        )}
      </div>
    </Main>
  )
}
