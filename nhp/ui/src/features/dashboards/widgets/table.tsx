/**
 * Flat table widget (DASHBOARDS-SCOPE §8 — DOM, not a chart). The gateway page's
 * network list (device count vs `max_devices`) renders through this. Pure
 * renderer of a TableWidget; empty → <Empty>.
 */
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import type { TableWidget } from './types'
import { Empty } from './empty'

export function TableTile({ widget }: { widget: TableWidget }) {
  if (widget.rows.length === 0) return <Empty />
  return (
    <Table>
      <TableHeader>
        <TableRow>
          {widget.columns.map((c) => (
            <TableHead key={c.key}>{c.label}</TableHead>
          ))}
        </TableRow>
      </TableHeader>
      <TableBody>
        {widget.rows.map((row, i) => (
          <TableRow key={i}>
            {widget.columns.map((c) => (
              <TableCell key={c.key}>{row[c.key] ?? '—'}</TableCell>
            ))}
          </TableRow>
        ))}
      </TableBody>
    </Table>
  )
}
