import {
  type ColumnDef,
  flexRender,
  getCoreRowModel,
  getSortedRowModel,
  type SortingState,
  useReactTable,
} from '@tanstack/react-table'
import { isNil, isObject } from 'lodash'
import React, { useMemo, useState } from 'react'

import { type TableColumnConfig } from '@/components/chart-builder/types'
import { type ColumnInfo } from '@/components/chart-builder/utils'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import { cn } from '@/lib/cn'

// Lean replacement for Laminar's infinite-datatable-backed TableChart. Renders a
// query result as a sortable table on the same TanStack Table primitive the
// admin grids use. Infinite scroll and persisted column config (their
// `table-config-store`) are intentionally dropped for now — they belong with the
// dashboard's debounced PATCH work (§2), not the chart rendering layer. The prop
// shape is kept compatible with `ChartRendererCore` so wiring them in later is
// additive.

interface TableChartProps {
  data: Record<string, any>[]
  columns: ColumnInfo[]
  hiddenColumns?: string[]
  onRowClick?: (rowData: Record<string, any>) => void
  // Accepted for call-site compatibility; not yet honored (see note above).
  tableColumnConfig?: TableColumnConfig
  onColumnConfigChange?: (config: TableColumnConfig) => void
  hasMore?: boolean
  isFetching?: boolean
  fetchNextPage?: () => void
}

/** Render any cell value as a string — objects/arrays as compact JSON. */
const renderCell = (value: unknown): string => {
  if (isNil(value)) return ''
  if (isObject(value)) return JSON.stringify(value)
  return String(value)
}

const TableChart = ({
  data,
  columns,
  hiddenColumns,
  onRowClick,
}: TableChartProps) => {
  const [sorting, setSorting] = useState<SortingState>([])

  const visibleColumns = useMemo(() => {
    const hidden = new Set(hiddenColumns ?? [])
    return columns.filter((c) => !hidden.has(c.name))
  }, [columns, hiddenColumns])

  const columnDefs = useMemo<ColumnDef<Record<string, any>>[]>(
    () =>
      visibleColumns.map((col) => ({
        accessorKey: col.name,
        header: col.name,
        cell: (ctx) => renderCell(ctx.getValue()),
      })),
    [visibleColumns],
  )

  const table = useReactTable({
    data,
    columns: columnDefs,
    state: { sorting },
    onSortingChange: setSorting,
    getCoreRowModel: getCoreRowModel(),
    getSortedRowModel: getSortedRowModel(),
  })

  return (
    <div className="h-full w-full overflow-auto">
      <Table>
        <TableHeader>
          {table.getHeaderGroups().map((hg) => (
            <TableRow key={hg.id}>
              {hg.headers.map((header) => (
                <TableHead
                  key={header.id}
                  className="cursor-pointer select-none whitespace-nowrap"
                  onClick={header.column.getToggleSortingHandler()}
                >
                  {flexRender(header.column.columnDef.header, header.getContext())}
                  {{ asc: ' ↑', desc: ' ↓' }[header.column.getIsSorted() as string] ?? ''}
                </TableHead>
              ))}
            </TableRow>
          ))}
        </TableHeader>
        <TableBody>
          {table.getRowModel().rows.map((row) => (
            <TableRow
              key={row.id}
              className={cn(onRowClick && 'cursor-pointer hover:bg-muted/40')}
              onClick={onRowClick ? () => onRowClick(row.original) : undefined}
            >
              {row.getVisibleCells().map((cell) => (
                <TableCell key={cell.id} className="whitespace-nowrap font-mono text-xs">
                  {flexRender(cell.column.columnDef.cell, cell.getContext())}
                </TableCell>
              ))}
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </div>
  )
}

export default TableChart
