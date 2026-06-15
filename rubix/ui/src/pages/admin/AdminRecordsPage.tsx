// Admin · Records browser + CRUD — the generic grid over /records. Lists every
// record the principal can read (optionally narrowed to one kind), and creates,
// edits, or deletes any of them. It renders whatever kinds the data contains and
// hardcodes none — a site, a task, and a device are all just records here.

import { getRouteApi } from '@tanstack/react-router'
import { useMemo, useState } from 'react'
import {
  flexRender,
  getCoreRowModel,
  getFilteredRowModel,
  useReactTable,
  type ColumnDef,
} from '@tanstack/react-table'
import { Pencil, Plus, Search, Trash2 } from 'lucide-react'
import { useAllRecords, useCollections } from '../../hooks/useAdmin'
import { useRecordMutations } from '../../hooks/useAdminMutations'
import { AdminLayout } from '../../components/admin/AdminLayout'
import { RecordDialog } from '../../components/admin/RecordDialog'
import { ErrorView, LoadingView, EmptyView } from '../../components/ui/StateView'
import { Button } from '../../components/ui/button'
import { Input } from '../../components/ui/input'
import { Badge } from '../../components/ui/badge'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '../../components/ui/select'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '../../components/ui/table'
import { useToast } from '../../components/ui/toast'
import type { Record, RecordContent } from '../../types/Record'

const route = getRouteApi('/t/$tenant/admin/records')
const ALL_KINDS = '__all__'

export function AdminRecordsPage() {
  const { tenant } = route.useParams()
  const [kind, setKind] = useState<string>(ALL_KINDS)
  const records = useAllRecords(tenant, kind === ALL_KINDS ? undefined : kind)
  const collections = useCollections(tenant)
  const { create, update, remove } = useRecordMutations(tenant)
  const { toast } = useToast()

  const [filter, setFilter] = useState('')
  const [editing, setEditing] = useState<Record | undefined>()
  const [dialogOpen, setDialogOpen] = useState(false)

  // The kinds offered in the filter: registered collections plus any kind seen in
  // the data, so an unregistered kind is still selectable.
  const kindOptions = useMemo(() => {
    const set = new Set<string>()
    for (const c of collections.data ?? []) set.add(c.name)
    for (const r of records.data ?? []) {
      if (typeof r.content?.kind === 'string' && r.content.kind) set.add(r.content.kind)
    }
    return [...set].sort()
  }, [collections.data, records.data])

  const columns = useMemo<ColumnDef<Record>[]>(
    () => [
      {
        header: 'Kind',
        accessorFn: (r) => (typeof r.content?.kind === 'string' ? r.content.kind : '(unkinded)'),
        cell: (ctx) => <span className="mono text-xs">{ctx.getValue<string>()}</span>,
      },
      {
        header: 'ID',
        accessorKey: 'id',
        cell: (ctx) => <span className="mono text-xs text-muted-foreground">{ctx.getValue<string>()}</span>,
      },
      {
        header: 'Content',
        accessorFn: (r) => summarize(r.content),
        cell: (ctx) => (
          <span className="mono max-w-[360px] truncate text-xs text-muted-foreground">
            {ctx.getValue<string>()}
          </span>
        ),
      },
      {
        header: 'Tags',
        accessorFn: (r) => r.tags.join(' '),
        cell: (ctx) => {
          const tags = ctx.row.original.tags
          if (tags.length === 0) return <span className="text-xs text-muted-foreground">—</span>
          return (
            <span className="flex flex-wrap gap-1">
              {tags.map((t) => (
                <Badge key={t} variant="muted" className="text-[10px]">
                  {t}
                </Badge>
              ))}
            </span>
          )
        },
      },
      {
        id: 'actions',
        header: '',
        cell: (ctx) => (
          <div className="flex justify-end gap-1">
            <Button
              variant="ghost"
              size="icon"
              onClick={() => openEdit(ctx.row.original)}
              aria-label="Edit record"
            >
              <Pencil size={14} />
            </Button>
            <Button
              variant="ghost"
              size="icon"
              onClick={() => handleDelete(ctx.row.original)}
              aria-label="Delete record"
            >
              <Trash2 size={14} className="text-destructive" />
            </Button>
          </div>
        ),
      },
    ],
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [],
  )

  const table = useReactTable({
    data: records.data ?? [],
    columns,
    state: { globalFilter: filter },
    onGlobalFilterChange: setFilter,
    getCoreRowModel: getCoreRowModel(),
    getFilteredRowModel: getFilteredRowModel(),
  })

  function openCreate() {
    setEditing(undefined)
    setDialogOpen(true)
  }
  function openEdit(record: Record) {
    setEditing(record)
    setDialogOpen(true)
  }

  async function handleSubmit(content: RecordContent) {
    try {
      if (editing) {
        await update.mutateAsync({ id: editing.id, content })
        toast('Record updated')
      } else {
        await create.mutateAsync(content)
        toast('Record created')
      }
      setDialogOpen(false)
    } catch (e) {
      toast(e instanceof Error ? e.message : 'Write failed', 'error')
    }
  }

  async function handleDelete(record: Record) {
    if (!window.confirm(`Delete record ${record.id}? This cannot be undone.`)) return
    try {
      await remove.mutateAsync(record.id)
      toast('Record deleted')
    } catch (e) {
      toast(e instanceof Error ? e.message : 'Delete failed', 'error')
    }
  }

  return (
    <AdminLayout active="records">
      <div className="mx-auto max-w-[1100px]">
        <div className="mb-5 flex items-center justify-between gap-3">
          <div>
            <h1 className="text-[22px] font-semibold tracking-tight">Records</h1>
            <div className="text-[13px] text-muted-foreground">
              Every record this principal can read in {tenant}.
            </div>
          </div>
          <Button onClick={openCreate} className="gap-1.5">
            <Plus size={16} /> New record
          </Button>
        </div>

        <div className="mb-4 flex items-center gap-3">
          <div className="relative">
            <Search size={15} className="absolute left-3 top-1/2 -translate-y-1/2 text-muted-foreground" />
            <Input
              value={filter}
              onChange={(e) => setFilter(e.target.value)}
              placeholder="Filter records…"
              className="w-[260px] pl-9"
            />
          </div>
          <Select value={kind} onValueChange={setKind}>
            <SelectTrigger className="w-[200px]">
              <SelectValue placeholder="All kinds" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value={ALL_KINDS}>All kinds</SelectItem>
              {kindOptions.map((k) => (
                <SelectItem key={k} value={k}>
                  {k}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <span className="ml-auto text-xs text-muted-foreground">
            {table.getFilteredRowModel().rows.length} shown
          </span>
        </div>

        {records.isLoading && <LoadingView label="Loading records…" />}
        {records.error && <ErrorView error={records.error} />}
        {records.data && records.data.length === 0 && (
          <EmptyView title="No records" hint="Create one, or boot the backend with SEED=1." />
        )}

        {records.data && records.data.length > 0 && (
          <div className="rounded-xl border border-border bg-card/40">
            <Table>
              <TableHeader>
                {table.getHeaderGroups().map((hg) => (
                  <TableRow key={hg.id}>
                    {hg.headers.map((h) => (
                      <TableHead key={h.id}>
                        {h.isPlaceholder ? null : flexRender(h.column.columnDef.header, h.getContext())}
                      </TableHead>
                    ))}
                  </TableRow>
                ))}
              </TableHeader>
              <TableBody>
                {table.getRowModel().rows.map((row) => (
                  <TableRow key={row.id}>
                    {row.getVisibleCells().map((cell) => (
                      <TableCell key={cell.id}>
                        {flexRender(cell.column.columnDef.cell, cell.getContext())}
                      </TableCell>
                    ))}
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </div>
        )}
      </div>

      <RecordDialog
        open={dialogOpen}
        onOpenChange={setDialogOpen}
        record={editing}
        onSubmit={handleSubmit}
        saving={create.isPending || update.isPending}
      />
    </AdminLayout>
  )
}

/** A one-line preview of a record's content, excluding the `kind` discriminator. */
function summarize(content: RecordContent): string {
  const { kind: _kind, ...rest } = content
  const json = JSON.stringify(rest)
  return json === '{}' ? '—' : json
}
