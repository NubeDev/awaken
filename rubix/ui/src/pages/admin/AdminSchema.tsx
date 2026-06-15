// Admin · Schema inspector — "how is this backend shaped?" for developers. Reads
// every record and any registered collections, then derives the structure purely
// from the data: what kinds exist, each kind's field shapes (declared where a
// collection registers them, inferred otherwise), and the tag graph. It names no
// domain type — point it at any rubix backend and it explains that backend.

import { getRouteApi } from '@tanstack/react-router'
import { useMemo, useState } from 'react'
import { Database, Hash, Tag } from 'lucide-react'
import { useAllRecords, useCollections } from '../../hooks/useAdmin'
import { profileKinds, tagFrequencies, type KindProfile } from '../../utils/schema'
import { AdminLayout } from '../../components/admin/AdminLayout'
import { ErrorView, LoadingView, EmptyView } from '../../components/ui/StateView'
import { Badge } from '../../components/ui/badge'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '../../components/ui/table'
import { cn } from '@/lib/cn'

const route = getRouteApi('/t/$tenant/admin/schema')

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

  return (
    <AdminLayout active="schema">
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
            {active && <KindDetail profile={active} />}
          </div>
        )}

        {tags.length > 0 && <TagGraph tags={tags} />}
      </div>
    </AdminLayout>
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

function KindDetail({ profile }: { profile: KindProfile }) {
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
        {profile.hasCollection ? (
          <Badge variant="default">registered collection</Badge>
        ) : (
          <Badge variant="muted">inferred shape</Badge>
        )}
      </div>
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
