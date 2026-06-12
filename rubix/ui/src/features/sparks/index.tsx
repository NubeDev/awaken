import { useMemo, useState } from 'react'
import { Check } from 'lucide-react'
import { useAckSpark, useSparks } from '@/api/hooks'
import type { SparkSeverity } from '@/api/types'
import { ConfigDrawer } from '@/components/config-drawer'
import { Header } from '@/components/layout/header'
import { Main } from '@/components/layout/main'
import { ProfileDropdown } from '@/components/profile-dropdown'
import { Search } from '@/components/search'
import { ThemeSwitch } from '@/components/theme-switch'
import { Button } from '@/components/ui/button'
import { Card, CardContent } from '@/components/ui/card'
import { Skeleton } from '@/components/ui/skeleton'
import { Tabs, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { useActiveSite } from '@/hooks/use-active-site'
import { SparkRow } from './components/spark-row'

type Filter = 'all' | SparkSeverity | 'open'

export function Sparks() {
  const { site } = useActiveSite()
  const { data: sparks = [], isLoading } = useSparks(site?.id)
  const ack = useAckSpark(site?.id)
  const [filter, setFilter] = useState<Filter>('open')

  const filtered = useMemo(() => {
    const sorted = [...sparks].sort((a, b) => b.ts.localeCompare(a.ts))
    if (filter === 'all') return sorted
    if (filter === 'open') return sorted.filter((s) => !s.acknowledged)
    return sorted.filter((s) => s.severity === filter)
  }, [sparks, filter])

  const openCount = sparks.filter((s) => !s.acknowledged).length

  return (
    <>
      <Header>
        <Search />
        <div className='ms-auto flex items-center gap-2'>
          <ThemeSwitch />
          <ConfigDrawer />
          <ProfileDropdown />
        </div>
      </Header>
      <Main>
        <div className='mb-4 flex items-center justify-between'>
          <div>
            <h1 className='text-2xl font-bold tracking-tight'>Sparks</h1>
            <p className='text-muted-foreground text-sm'>
              {openCount} open finding{openCount === 1 ? '' : 's'}
              {site ? ` · ${site.display_name}` : ''}
            </p>
          </div>
        </div>

        <Tabs value={filter} onValueChange={(v) => setFilter(v as Filter)} className='mb-4'>
          <TabsList>
            <TabsTrigger value='open'>Open</TabsTrigger>
            <TabsTrigger value='fault'>Fault</TabsTrigger>
            <TabsTrigger value='warning'>Warning</TabsTrigger>
            <TabsTrigger value='info'>Info</TabsTrigger>
            <TabsTrigger value='all'>All</TabsTrigger>
          </TabsList>
        </Tabs>

        <Card>
          <CardContent className='p-2'>
            {isLoading ? (
              <div className='space-y-2 p-1'>
                {Array.from({ length: 6 }).map((_, i) => (
                  <Skeleton key={i} className='h-14 rounded-lg' />
                ))}
              </div>
            ) : filtered.length === 0 ? (
              <p className='text-muted-foreground py-12 text-center text-sm'>No findings here.</p>
            ) : (
              <ul className='divide-border divide-y'>
                {filtered.map((s) => (
                  <li key={s.id} className='flex items-center gap-2'>
                    <div className='flex-1'>
                      <SparkRow spark={s} />
                    </div>
                    {!s.acknowledged && (
                      <Button
                        variant='ghost'
                        size='sm'
                        className='me-2 shrink-0'
                        disabled={ack.isPending}
                        onClick={() => ack.mutate(s.id)}
                      >
                        <Check className='size-3.5' /> Ack
                      </Button>
                    )}
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
