import { useNavigate } from '@tanstack/react-router'
import { useSparks } from '@/api/hooks'
import type { Uuid } from '@/api/types'
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card'
import { Skeleton } from '@/components/ui/skeleton'
import { SparkRow } from '@/features/sparks/components/spark-row'

/** Latest rule findings for the active site, linking into the Sparks surface. */
export function RecentSparks({ siteId }: { siteId: Uuid | undefined }) {
  const navigate = useNavigate()
  const { data: sparks = [], isLoading } = useSparks(siteId)
  const recent = [...sparks].sort((a, b) => b.ts.localeCompare(a.ts)).slice(0, 6)

  return (
    <Card>
      <CardHeader>
        <CardTitle>Recent Sparks</CardTitle>
        <CardDescription>Latest rule findings</CardDescription>
      </CardHeader>
      <CardContent>
        {isLoading ? (
          <div className='space-y-2'>
            {Array.from({ length: 4 }).map((_, i) => (
              <Skeleton key={i} className='h-12 rounded-lg' />
            ))}
          </div>
        ) : recent.length === 0 ? (
          <p className='text-muted-foreground py-8 text-center text-sm'>No open findings.</p>
        ) : (
          <div className='-mx-1 flex flex-col'>
            {recent.map((s) => (
              <SparkRow key={s.id} spark={s} onClick={() => navigate({ to: '/sparks' })} />
            ))}
          </div>
        )}
      </CardContent>
    </Card>
  )
}
