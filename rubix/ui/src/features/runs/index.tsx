import { useRuns } from '@/api/hooks'
import { Main } from '@/components/layout/main'
import { PageHeader } from '@/components/layout/page-header'
import { Badge } from '@/components/ui/badge'
import { Card, CardContent } from '@/components/ui/card'
import { Skeleton } from '@/components/ui/skeleton'
import { relativeTime } from '@/lib/format'

/** awaken agent run records, read live from `/api/v1/runs`. */
export function Runs() {
  const { data: runs = [], isLoading } = useRuns()

  return (
    <>
      <PageHeader title='Agent Runs' sub='awaken activity & approvals' />
      <Main fluid>
        <Card>
          <CardContent className='p-2'>
            {isLoading ? (
              <div className='space-y-2 p-1'>
                {Array.from({ length: 4 }).map((_, i) => (
                  <Skeleton key={i} className='h-12 rounded-lg' />
                ))}
              </div>
            ) : runs.length === 0 ? (
              <p className='text-muted-foreground py-12 text-center text-sm'>No agent runs yet.</p>
            ) : (
              <ul className='divide-border divide-y'>
                {runs.map((r) => (
                  <li key={r.id} className='flex items-center gap-3 px-2.5 py-3'>
                    <div className='min-w-0 flex-1'>
                      <div className='truncate text-[13px] font-medium'>{r.response || r.id}</div>
                      <div className='text-muted-foreground font-mono text-[11px]'>{r.id}</div>
                    </div>
                    <span className='text-muted-foreground text-[11px]'>
                      {relativeTime(r.created_at)}
                    </span>
                    <Badge variant={r.status === 'suspended' ? 'warning' : 'muted'}>
                      {r.status}
                    </Badge>
                  </li>
                ))}
              </ul>
            )}
          </CardContent>
        </Card>
      </Main>
    </>
  )
}
