// Admin · Schema inspector — "how is this backend shaped?" for developers. Reads
// every record and any registered collections, then derives the structure purely
// from the data: what kinds exist, each kind's field shapes (declared where a
// collection registers them, inferred otherwise), and the tag graph. It names no
// domain type — point it at any rubix backend and it explains that backend.

import { getRouteApi } from '@tanstack/react-router'
import { useMemo, useState } from 'react'
import { useMutation } from '@tanstack/react-query'
import { Database, Hash, Play, Tag } from 'lucide-react'
import { useAllRecords, useCollections } from '../../../hooks/useAdmin'
import { profileKinds, tagFrequencies, type KindProfile } from '../../../utils/schema'
import { useApi } from '../../../api/ConnectionContext'
import { runQuery, type QueryResponse } from '../../../api/query'
import { usePageHeader } from '../../../components/shell/page-header'
import { ErrorView, LoadingView, EmptyView } from '../../../components/ui/StateView'
import { Badge } from '../../../components/ui/badge'
import { Button } from '../../../components/ui/button'
import { SqlEditor } from '../../../components/sql/SqlEditor'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '../../../components/ui/table'
import { cn } from '@/lib/cn'

const route = getRouteApi('/t/$tenant/admin/schema')

// The `kind` discriminator lives at content.content.kind in the record table
// (the outer `content` column wraps the document, whose own `content` holds the
// payload) — the same json_get path the Query console's "Records by kind" preset
// uses. '(unkinded)' is the schema page's bucket for records with no kind, so it
// queries the whole table rather than filtering.
function queryForKind(kind: string): string {
  const base = 'SELECT id, content, tags, created FROM record'
  if (kind === '(unkinded)') return `${base}\nORDER BY created DESC\nLIMIT 50`
  const path = "json_get(json_get(content, 'content'), 'kind')"
  return `${base}\nWHERE ${path} = '${kind.replace(/'/g, "''")}'\nORDER BY created DESC\nLIMIT 50`
}

export function AdminSchema() {
  const { tenant } = route.useParams()
  const records = useAllRecords(tenant)
  const collections = useCollections(tenant)

  const profiles = useMemo(
    () => profileKinds(records.data ?? [], collections.data ?? []),
    [records.data, collections.data],
  )
  const tags = useMemo(() => tagFrequencies(records.data ?? []), [records.data])
  const [activeKind, setActiveKind] = useState<string | null>(null)

  const active = profiles.find((p) => p.kind === activeKind) ?? profiles[0] ?? null

  usePageHeader({ crumbs: ['Admin', 'Schema'] })

  return (
    <div className="px-6 py-6">
      <div className="mx-auto max-w-[1100px]">
        <Header count={records.data?.length} kinds={profiles.length} />

        {records.isLoading && <LoadingView label="Inspecting the backend…" />}
        {records.error && <ErrorView error={records.error} />}

        {records.data && profiles.length === 0 && (
          <EmptyView title="No records yet" hint="Boot the backend with SEED=1 to populate a demo dataset, or create a record." />
        )}

        {records.data && profiles.length > 0 && (
          <div className="grid grid-cols-[260px_1fr] gap-5">
            <KindList profiles={profiles} active={active} onSelect={setActiveKind} />
            {/* Remount the detail (and its query panel) per kind so switching
                kinds clears any open results and resets the SQL to that kind. */}
            {active && <KindDetail key={active.kind} profile={active} tenant={tenant} />}
          </div>
        )}

        {tags.length > 0 && <TagGraph tags={tags} />}
      </div>
    </div>
  )
}

function Header({ count, kinds }: { count?: number; kinds: number }) {
  return (
    <div className="mb-6 flex items-center gap-3">
      <div className="grid size-11 place-items-center rounded-xl border border-border bg-card">
        <Database size={20} className="text-muted-foreground" />
      </div>
      <div>
        <h1 className="text-[22px] font-semibold tracking-tight">Schema</h1>
        <div className="text-[13px] text-muted-foreground">
          {count ?? 0} records across {kinds} {kinds === 1 ? 'kind' : 'kinds'} — derived live from the data.
        </div>
      </div>
    </div>
  )
}

function KindList({
  profiles,
  active,
  onSelect,
}: {
  profiles: KindProfile[]
  active: KindProfile | null
  onSelect: (kind: string) => void
}) {
  return (
    <div className="flex flex-col gap-1">
      {profiles.map((p) => (
        <button
          key={p.kind}
          type="button"
          onClick={() => onSelect(p.kind)}
          className={cn(
            'flex items-center justify-between rounded-lg border px-3 py-2 text-left text-sm transition-colors',
            p.kind === active?.kind
              ? 'border-primary/40 bg-primary/10 text-foreground'
              : 'border-border bg-card/40 text-muted-foreground hover:bg-muted',
          )}
        >
          <span className="flex items-center gap-2">
            <span className="mono font-medium text-foreground">{p.kind}</span>
            {p.hasCollection && (
              <Badge variant="default" className="text-[10px]">
                collection
              </Badge>
            )}
          </span>
          <span className="mono text-xs text-muted-foreground">{p.count}</span>
        </button>
      ))}
    </div>
  )
}

function KindDetail({ profile, tenant }: { profile: KindProfile; tenant: string }) {
  const [querying, setQuerying] = useState(false)
  return (
    <div className="rounded-xl border border-border bg-card/40">
      <div className="flex items-center justify-between border-b border-border px-4 py-3">
        <div className="flex items-center gap-2">
          <Hash size={15} className="text-muted-foreground" />
          <span className="mono text-sm font-semibold">{profile.kind}</span>
          <span className="text-xs text-muted-foreground">
            · {profile.count} {profile.count === 1 ? 'record' : 'records'}
          </span>
        </div>
        <div className="flex items-center gap-2">
          <Button
            variant="outline"
            onClick={() => setQuerying((q) => !q)}
            className="h-7 gap-1.5 text-xs"
          >
            <Play size={13} /> {querying ? 'Hide query' : 'Query this kind'}
          </Button>
          {profile.hasCollection ? (
            <Badge variant="default">registered collection</Badge>
          ) : (
            <Badge variant="muted">inferred shape</Badge>
          )}
        </div>
      </div>
      {querying && <QueryPanel kind={profile.kind} tenant={tenant} />}
      {profile.fields.length === 0 ? (
        <div className="px-4 py-6 text-center text-sm text-muted-foreground">
          No fields beyond <span className="mono">kind</span>.
        </div>
      ) : (
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Field</TableHead>
              <TableHead>Type</TableHead>
              <TableHead>Presence</TableHead>
              <TableHead>Sample</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {profile.fields.map((f) => (
              <TableRow key={f.name}>
                <TableCell className="mono font-medium">
                  <span className="flex items-center gap-2">
                    {f.name}
                    {f.declared && (
                      <Badge variant="default" className="text-[10px]">
                        declared
                      </Badge>
                    )}
                  </span>
                </TableCell>
                <TableCell className="mono text-xs text-muted-foreground">
                  {f.declaredType ?? (f.types.join(' | ') || '—')}
                </TableCell>
                <TableCell>
                  <PresenceBar fraction={f.presence} />
                </TableCell>
                <TableCell className="mono max-w-[280px] truncate text-xs text-muted-foreground">
                  {f.sample}
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      )}
    </div>
  )
}

// Inline query tool: run a read-only SELECT scoped to the kind being inspected,
// against POST /query (the same DataFusion surface as the Query console). The SQL
// is pre-filled and editable — the schema page is where you learn a kind's shape,
// so it's also where you should be able to pull its rows without leaving.
function QueryPanel({ kind, tenant }: { kind: string; tenant: string }) {
  const api = useApi(tenant)
  const [sql, setSql] = useState(() => queryForKind(kind))

  const query = useMutation<QueryResponse, Error, string>({
    mutationFn: (text) => runQuery(api, text),
  })

  const rows = query.data?.rows ?? []
  const columns = useMemo(() => {
    const set = new Set<string>()
    for (const row of rows) for (const key of Object.keys(row)) set.add(key)
    return [...set]
  }, [rows])

  function run() {
    if (sql.trim()) query.mutate(sql)
  }

  return (
    <div className="border-b border-border bg-bg/30 px-4 py-3">
      <SqlEditor value={sql} onChange={setSql} onRun={run} minHeight="96px" />
      <div className="mt-2 flex items-center gap-3">
        <Button onClick={run} disabled={query.isPending} className="h-7 gap-1.5 text-xs">
          <Play size={13} /> {query.isPending ? 'Running…' : 'Run'}
        </Button>
        <span className="text-[11px] text-muted-foreground">⌘/Ctrl + Enter</span>
        {query.data && (
          <span className="ml-auto text-[11px] text-muted-foreground">
            {rows.length} {rows.length === 1 ? 'row' : 'rows'}
          </span>
        )}
      </div>

      {query.error && (
        <div className="mt-3">
          <ErrorView error={query.error} />
        </div>
      )}

      {query.data &&
        (rows.length === 0 ? (
          <div className="mt-3 rounded-lg border border-border bg-card/40 px-4 py-5 text-center text-xs text-muted-foreground">
            No rows returned.
          </div>
        ) : (
          <div className="mt-3 max-h-[360px] overflow-auto rounded-lg border border-border bg-card/40">
            <Table>
              <TableHeader>
                <TableRow>
                  {columns.map((c) => (
                    <TableHead key={c} className="mono">
                      {c}
                    </TableHead>
                  ))}
                </TableRow>
              </TableHeader>
              <TableBody>
                {rows.map((row, i) => (
                  <TableRow key={i}>
                    {columns.map((c) => (
                      <TableCell key={c} className="mono max-w-[320px] truncate text-xs">
                        {renderCell(row[c])}
                      </TableCell>
                    ))}
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </div>
        ))}
    </div>
  )
}

function renderCell(value: unknown): string {
  if (value === null || value === undefined) return '—'
  if (typeof value === 'object') return JSON.stringify(value)
  return String(value)
}

function PresenceBar({ fraction }: { fraction: number }) {
  const pct = Math.round(fraction * 100)
  return (
    <span className="flex items-center gap-2">
      <span className="h-1.5 w-16 overflow-hidden rounded-full bg-muted">
        <span
          className={cn('block h-full rounded-full', pct === 100 ? 'bg-green' : 'bg-amber')}
          style={{ width: `${pct}%` }}
        />
      </span>
      <span className="mono text-[11px] text-muted-foreground">{pct}%</span>
    </span>
  )
}

function TagGraph({ tags }: { tags: { tag: string; count: number }[] }) {
  return (
    <div className="mt-8">
      <div className="mb-3 flex items-center gap-2">
        <Tag size={15} className="text-muted-foreground" />
        <h2 className="text-sm font-semibold">Tags</h2>
        <span className="text-xs text-muted-foreground">
          {tags.length} distinct — the structure-by-tagging graph.
        </span>
      </div>
      <div className="flex flex-wrap gap-2">
        {tags.map((t) => (
          <span
            key={t.tag}
            className="flex items-center gap-1.5 rounded-lg border border-border bg-card/40 px-2.5 py-1 text-xs"
          >
            <span className="mono">{t.tag}</span>
            <span className="mono text-[10px] text-muted-foreground">{t.count}</span>
          </span>
        ))}
      </div>
    </div>
  )
}
