// <RecordGrid> — the load-bearing admin primitive: render any record set as
// columns, filtered by kind (ADMIN-UI taxonomy). Bound to the generic /records
// surface, so the same component drives the admin browser and any domain grid.
// Filtering is client-side for now (ADMIN-UI open question 2).

import { useMemo, useState } from 'react'
import { ChevronRight, Search } from 'lucide-react'
import type { Record } from '../../types/Record'
import { relTime } from '../../utils/format'

const str = (v: unknown): string => (typeof v === 'string' ? v : '')

export function RecordGrid({ records }: { records: Record[] }) {
  const [kind, setKind] = useState<string>('all')
  const [q, setQ] = useState('')
  const [open, setOpen] = useState<string | null>(null)

  const kinds = useMemo(() => {
    const set = new Set<string>()
    for (const r of records) if (r.content?.kind) set.add(str(r.content.kind))
    return ['all', ...Array.from(set).sort()]
  }, [records])

  const rows = useMemo(() => {
    const ql = q.toLowerCase().trim()
    return records.filter((r) => {
      if (kind !== 'all' && r.content?.kind !== kind) return false
      if (!ql) return true
      return r.id.toLowerCase().includes(ql) || JSON.stringify(r.content).toLowerCase().includes(ql)
    })
  }, [records, kind, q])

  return (
    <div>
      <div className="flex items-center gap-2 mb-3 flex-wrap">
        <div className="flex items-center gap-1.5 flex-wrap">
          {kinds.map((k) => (
            <button
              key={k}
              onClick={() => setKind(k)}
              className={`chip rounded-full border px-3 py-1.5 text-[12.5px] whitespace-nowrap ${
                kind === k ? 'border-r1/40 bg-r1/[.08] text-fg' : 'border-border bg-panel2 text-muted'
              }`}
            >
              {k}
            </button>
          ))}
        </div>
        <div className="ml-auto relative">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 text-muted" size={14} />
          <input
            value={q}
            onChange={(e) => setQ(e.target.value)}
            placeholder="Filter records…"
            className="h-9 w-[240px] rounded-lg border border-border bg-panel2 pl-9 pr-3 text-[13px] outline-none placeholder:text-muted focus:border-r1/50 transition"
          />
        </div>
      </div>

      <div className="rounded-2xl border border-border bg-panel2 overflow-hidden">
        <div className="grid grid-cols-[120px_1fr_140px_40px] gap-3 px-4 py-2.5 border-b border-border text-[11px] uppercase tracking-wider text-muted font-medium">
          <span>Kind</span>
          <span>Name / id</span>
          <span>Updated</span>
          <span />
        </div>
        <div className="divide-y divide-border max-h-[560px] overflow-auto">
          {rows.map((r) => (
            <RecordRow key={r.id} record={r} open={open === r.id} onToggle={() => setOpen(open === r.id ? null : r.id)} />
          ))}
          {rows.length === 0 && <div className="px-4 py-8 text-center text-[13px] text-muted">No matching records.</div>}
        </div>
      </div>
      <div className="text-[11.5px] text-muted mt-2 mono">
        {rows.length} of {records.length} records
      </div>
    </div>
  )
}

function RecordRow({ record, open, onToggle }: { record: Record; open: boolean; onToggle: () => void }) {
  const name = str(record.content?.name) || str(record.content?.key) || record.id
  const kind = str(record.content?.kind) || '—'
  return (
    <div>
      <button onClick={onToggle} className="w-full grid grid-cols-[120px_1fr_140px_40px] gap-3 px-4 py-3 text-left hover:bg-panel3/60 transition items-center">
        <span className="text-[12px] mono text-muted truncate">{kind}</span>
        <span className="min-w-0">
          <span className="text-[13.5px] font-medium block truncate">{name}</span>
          <span className="text-[11px] text-muted mono block truncate">{record.id}</span>
        </span>
        <span className="text-[12px] text-muted mono">{relTime(record.updated)}</span>
        <ChevronRight size={16} className={`text-muted transition ${open ? 'rotate-90' : ''}`} />
      </button>
      {open && (
        <pre className="px-4 pb-4 text-[11.5px] mono text-fg/75 overflow-auto bg-bg/40">
          {JSON.stringify(record.content, null, 2)}
        </pre>
      )}
    </div>
  )
}
