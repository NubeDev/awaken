// Admin · Audit log — the immutable "who did what, when" trail for THIS tenant.
// Each mutating command appends one append-only audit row (rubix-gate audit,
// SCOPE.md "Audit log"); this page reads them back, newest first, with the
// before/after summary expandable per row.
//
// Tenant scoping is NOT done here: the `audit` table carries a SurrealDB
// row-permission `FOR select WHERE namespace = $auth.namespace`
// (rubix-gate audit/permit.rs), so a plain `SELECT … FROM audit` over the
// scoped session already returns only the current tenant's rows. The page never
// adds a namespace filter — doing so would be redundant and could mislead a
// reader into thinking scoping were the UI's job.

import { getRouteApi } from '@tanstack/react-router'
import { useMemo, useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { ScrollText, ChevronRight, RefreshCw } from 'lucide-react'
import { useApi } from '../../../api/ConnectionContext'
import { runQuery } from '../../../api/query'
import { usePageHeader } from '../../../components/shell/page-header'
import { ErrorView, LoadingView, EmptyView } from '../../../components/ui/StateView'
import { Badge } from '../../../components/ui/badge'
import { Button } from '../../../components/ui/button'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '../../../components/ui/select'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '../../../components/ui/table'
import { cn } from '@/lib/cn'

const route = getRouteApi('/t/$tenant/admin/audit')

// The audit fields live inside the `content` JSON column of the audit table
// (chart-presets.ts uses the same json_get path); structural columns id/created
// sit alongside. Newest-first, capped — an audit log is read tail-first.
const AUDIT_SQL = `SELECT
  id,
  created,
  json_get(content, 'action') AS action,
  json_get(content, 'subject') AS subject,
  json_get(content, 'target') AS target,
  json_get(content, 'correlation_id') AS correlation_id,
  json_get(content, 'before') AS before,
  json_get(content, 'after') AS after
FROM audit
ORDER BY created DESC
LIMIT 200`

interface AuditEvent {
  id: string
  created: string
  action: string
  subject: string
  target: string
  correlation_id: string
  before: unknown
  after: unknown
}

const ALL = '__all__'

export function AdminAudit() {
  const { tenant } = route.useParams()
  const api = useApi(tenant)
  const [action, setAction] = useState<string>(ALL)

  const audit = useQuery({
    queryKey: ['audit', tenant],
    queryFn: async () => {
      const res = await runQuery(api, AUDIT_SQL)
      return res.rows as unknown as AuditEvent[]
    },
  })

  const rows = audit.data ?? []
  const actions = useMemo(() => {
    const set = new Set<string>()
    for (const r of rows) if (r.action) set.add(String(r.action))
    return [...set].sort()
  }, [rows])
  const shown = action === ALL ? rows : rows.filter((r) => String(r.action) === action)

  usePageHeader({ crumbs: ['Admin', 'Audit'] })

  return (
    <div className="px-6 py-6">
      <div className="mx-auto max-w-[1100px]">
        <Header count={rows.length} />

        <div className="mb-3 flex flex-wrap items-center gap-2">
          <Select value={action} onValueChange={setAction}>
            <SelectTrigger className="w-[180px]">
              <SelectValue placeholder="All actions" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value={ALL}>All actions</SelectItem>
              {actions.map((a) => (
                <SelectItem key={a} value={a}>
                  {a}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <Button
            variant="outline"
            onClick={() => audit.refetch()}
            disabled={audit.isFetching}
            className="gap-1.5"
          >
            <RefreshCw size={15} className={cn(audit.isFetching && 'animate-spin')} /> Refresh
          </Button>
          {audit.data && (
            <span className="ml-auto text-xs text-muted-foreground">
              {shown.length} of {rows.length} {rows.length === 1 ? 'event' : 'events'}
              {rows.length === 200 && ' (latest 200)'}
            </span>
          )}
        </div>

        {audit.isLoading && <LoadingView label="Reading the audit trail…" />}
        {audit.error && <ErrorView error={audit.error} />}

        {audit.data && rows.length === 0 && (
          <EmptyView
            title="No audit events yet"
            hint="Every mutating action through the gate appends one immutable audit row. Make a change and it appears here."
          />
        )}

        {audit.data && shown.length > 0 && <AuditTable events={shown} />}
      </div>
    </div>
  )
}

function Header({ count }: { count: number }) {
  return (
    <div className="mb-6 flex items-center gap-3">
      <div className="grid size-11 place-items-center rounded-xl border border-border bg-card">
        <ScrollText size={20} className="text-muted-foreground" />
      </div>
      <div>
        <h1 className="text-[22px] font-semibold tracking-tight">Audit</h1>
        <div className="text-[13px] text-muted-foreground">
          Immutable who-did-what trail for this tenant — {count} {count === 1 ? 'event' : 'events'}, newest first.
        </div>
      </div>
    </div>
  )
}

function AuditTable({ events }: { events: AuditEvent[] }) {
  return (
    <div className="rounded-xl border border-border bg-card/40">
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead className="w-[180px]">When</TableHead>
            <TableHead className="w-[110px]">Action</TableHead>
            <TableHead>Subject</TableHead>
            <TableHead>Target</TableHead>
            <TableHead className="w-[40px]" />
          </TableRow>
        </TableHeader>
        <TableBody>
          {events.map((e) => (
            <AuditRowView key={e.id} event={e} />
          ))}
        </TableBody>
      </Table>
    </div>
  )
}

function AuditRowView({ event }: { event: AuditEvent }) {
  const [open, setOpen] = useState(false)
  const hasDetail = event.before != null || event.after != null

  return (
    <>
      <TableRow
        className={cn(hasDetail && 'cursor-pointer')}
        onClick={() => hasDetail && setOpen((o) => !o)}
      >
        <TableCell className="mono text-xs text-muted-foreground">{formatTime(event.created)}</TableCell>
        <TableCell>
          <Badge variant={actionVariant(event.action)} className="text-[10px]">
            {event.action ?? '—'}
          </Badge>
        </TableCell>
        <TableCell className="mono text-xs">{event.subject ?? '—'}</TableCell>
        <TableCell className="mono max-w-[280px] truncate text-xs text-muted-foreground">
          {event.target ?? '—'}
        </TableCell>
        <TableCell>
          {hasDetail && (
            <ChevronRight
              size={15}
              className={cn('text-muted-foreground transition-transform', open && 'rotate-90')}
            />
          )}
        </TableCell>
      </TableRow>
      {open && hasDetail && (
        <TableRow className="hover:bg-transparent">
          <TableCell colSpan={5} className="bg-bg/30 p-0">
            <div className="grid grid-cols-2 gap-3 px-4 py-3">
              <DiffColumn label="Before" value={event.before} />
              <DiffColumn label="After" value={event.after} />
            </div>
            {event.correlation_id && (
              <div className="border-t border-border px-4 py-2 text-[11px] text-muted-foreground">
                correlation:{' '}
                <span className="mono text-foreground">{String(event.correlation_id)}</span>
              </div>
            )}
          </TableCell>
        </TableRow>
      )}
    </>
  )
}

function DiffColumn({ label, value }: { label: string; value: unknown }) {
  return (
    <div>
      <div className="mb-1.5 text-[11px] font-medium uppercase tracking-wide text-muted-foreground">
        {label}
      </div>
      {value == null ? (
        <div className="rounded-md border border-dashed border-border px-3 py-2 text-xs text-muted-foreground">
          (none)
        </div>
      ) : (
        <pre className="max-h-[280px] overflow-auto rounded-md bg-bg/50 p-3 text-xs mono">
          {format(value)}
        </pre>
      )}
    </div>
  )
}

// The json_get of an object column may arrive as a JSON string or an already-
// parsed object depending on the column's type tag — render either as pretty JSON.
function format(value: unknown): string {
  if (typeof value === 'string') {
    try {
      return JSON.stringify(JSON.parse(value), null, 2)
    } catch {
      return value
    }
  }
  return JSON.stringify(value, null, 2)
}

function formatTime(value: string): string {
  const d = new Date(value)
  return Number.isNaN(d.getTime()) ? String(value) : d.toLocaleString()
}

function actionVariant(action: string): 'default' | 'muted' | 'destructive' {
  if (action === 'delete') return 'destructive'
  if (action === 'create') return 'default'
  return 'muted'
}
