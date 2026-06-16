/**
 * The final PREVIEW of records-to-create, shown before the batched write
 * (WIZARDS.md §Principles: "previews the records it will create, then writes
 * them"). After a run it doubles as the per-record RESULT view — each row shows
 * its `BatchResult` status so a partial failure is visible per record (resume,
 * not restart).
 *
 * Generic over any wizard's plan: it renders the `PlannedRecord` label + kind +
 * the standard `tags` the wizard applied (so the operator can see the tag
 * vocabulary that will drive dashboard auto-build). Pure presentation.
 */
import { Badge } from '@/components/ui/badge'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import type { BatchResults, PlannedRecord } from './batch-write'

const STATUS_VARIANT = {
  pending: 'secondary',
  ok: 'default',
  error: 'destructive',
} as const

export function PreviewTable({
  plan,
  results,
}: {
  plan: PlannedRecord[]
  /** Present after a run — adds a per-record status column. */
  results?: BatchResults
}) {
  return (
    <div className='max-h-[28rem] overflow-auto rounded-md border'>
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead className='w-40'>Kind</TableHead>
            <TableHead>Record</TableHead>
            <TableHead>Tags</TableHead>
            {results ? <TableHead className='w-28'>Status</TableHead> : null}
          </TableRow>
        </TableHeader>
        <TableBody>
          {plan.map((p) => {
            const tags = (p.content.tags as string[] | undefined) ?? []
            const r = results?.[p.id]
            return (
              <TableRow key={p.id}>
                <TableCell className='text-muted-foreground font-mono text-xs'>
                  {p.kind}
                </TableCell>
                <TableCell className='font-medium'>{p.label}</TableCell>
                <TableCell>
                  <div className='flex flex-wrap gap-1'>
                    {tags.map((t) => (
                      <Badge key={t} variant='outline' className='text-xs'>
                        {t}
                      </Badge>
                    ))}
                  </div>
                </TableCell>
                {results ? (
                  <TableCell>
                    <Badge variant={STATUS_VARIANT[r?.status ?? 'pending']}>
                      {r?.status ?? 'pending'}
                    </Badge>
                    {r?.error ? (
                      <p className='text-destructive mt-1 text-xs'>{r.error}</p>
                    ) : null}
                  </TableCell>
                ) : null}
              </TableRow>
            )
          })}
        </TableBody>
      </Table>
    </div>
  )
}
