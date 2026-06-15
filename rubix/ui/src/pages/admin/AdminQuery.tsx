// Admin · Query console — run a read-only query over POST /query (DataFusion) and
// render the rows. A developer's ad-hoc window into the data; gated server-side on
// the external-query capability. Renders whatever columns the result carries — no
// domain assumptions.

import { getRouteApi } from '@tanstack/react-router'
import { useMemo, useState } from 'react'
import { useMutation } from '@tanstack/react-query'
import { Play, TerminalSquare } from 'lucide-react'
import { useApi } from '../../api/ConnectionContext'
import { runQuery, type QueryResponse } from '../../api/query'
import { AdminLayout } from '../../components/admin/AdminLayout'
import { ErrorView } from '../../components/ui/StateView'
import { Button } from '../../components/ui/button'
import { Textarea } from '../../components/ui/textarea'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '../../components/ui/table'

const route = getRouteApi('/t/$tenant/admin/query')

const STARTER = "SELECT content.kind AS kind, count() AS n FROM record GROUP BY kind"

export function AdminQuery() {
  const { tenant } = route.useParams()
  const api = useApi(tenant)
  const [sql, setSql] = useState(STARTER)

  const query = useMutation<QueryResponse, Error, string>({
    mutationFn: (text) => runQuery(api, text),
  })

  const columns = useMemo(() => {
    const rows = query.data?.rows ?? []
    const set = new Set<string>()
    for (const row of rows) for (const key of Object.keys(row)) set.add(key)
    return [...set]
  }, [query.data])

  function run() {
    if (sql.trim()) query.mutate(sql)
  }

  return (
    <AdminLayout active="query">
      <div className="mx-auto max-w-[1100px]">
        <div className="mb-5 flex items-center gap-3">
          <div className="grid size-11 place-items-center rounded-xl border border-border bg-card">
            <TerminalSquare size={20} className="text-muted-foreground" />
          </div>
          <div>
            <h1 className="text-[22px] font-semibold tracking-tight">Query</h1>
            <div className="text-[13px] text-muted-foreground">
              Run a read-only query over the data plane.
            </div>
          </div>
        </div>

        <Textarea
          value={sql}
          onChange={(e) => setSql(e.target.value)}
          spellCheck={false}
          onKeyDown={(e) => {
            if ((e.metaKey || e.ctrlKey) && e.key === 'Enter') run()
          }}
          className="mono min-h-[120px] text-xs"
        />
        <div className="mt-3 flex items-center gap-3">
          <Button onClick={run} disabled={query.isPending} className="gap-1.5">
            <Play size={15} /> {query.isPending ? 'Running…' : 'Run'}
          </Button>
          <span className="text-xs text-muted-foreground">⌘/Ctrl + Enter</span>
          {query.data && (
            <span className="ml-auto text-xs text-muted-foreground">
              {query.data.rows.length} {query.data.rows.length === 1 ? 'row' : 'rows'}
            </span>
          )}
        </div>

        {query.error && (
          <div className="mt-5">
            <ErrorView error={query.error} />
          </div>
        )}

        {query.data && (
          <div className="mt-5 rounded-xl border border-border bg-card/40">
            {query.data.rows.length === 0 ? (
              <div className="px-4 py-6 text-center text-sm text-muted-foreground">
                No rows returned.
              </div>
            ) : (
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
                  {query.data.rows.map((row, i) => (
                    <TableRow key={i}>
                      {columns.map((c) => (
                        <TableCell key={c} className="mono text-xs">
                          {renderCell(row[c])}
                        </TableCell>
                      ))}
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            )}
          </div>
        )}
      </div>
    </AdminLayout>
  )
}

function renderCell(value: unknown): string {
  if (value === null || value === undefined) return '—'
  if (typeof value === 'object') return JSON.stringify(value)
  return String(value)
}
