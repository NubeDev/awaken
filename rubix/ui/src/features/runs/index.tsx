import { useRuns } from '@/api/hooks'
import { Main } from '@/components/layout/main'
import { PageHeader } from '@/components/layout/page-header'
import { Card, CardContent } from '@/components/ui/card'
import { Skeleton } from '@/components/ui/skeleton'
import { RunRow } from './components/run-row'

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
                  <RunRow key={r.id} run={r} />
                ))}
              </ul>
            )}
          </CardContent>
        </Card>
      </Main>
    </>
  )
}
