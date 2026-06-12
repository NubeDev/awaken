import { useState } from 'react'
import { useMutation } from '@tanstack/react-query'
import { Play } from 'lucide-react'
import * as api from '@/api/endpoints'
import { ApiError } from '@/api/client'
import { ConfigDrawer } from '@/components/config-drawer'
import { Header } from '@/components/layout/header'
import { Main } from '@/components/layout/main'
import { ProfileDropdown } from '@/components/profile-dropdown'
import { Search } from '@/components/search'
import { ThemeSwitch } from '@/components/theme-switch'
import { Button } from '@/components/ui/button'
import { Card, CardContent } from '@/components/ui/card'
import { Textarea } from '@/components/ui/textarea'
import { formatValue } from '@/lib/format'

const DEFAULT_SQL = 'SELECT slug, display_name, kind FROM points LIMIT 20'

/** DataFusion SQL surface — runs real `/api/v1/query` against the store. */
export function History() {
  const [sql, setSql] = useState(DEFAULT_SQL)
  const run = useMutation({ mutationFn: () => api.query.run(sql) })

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
        <div className='mb-4'>
          <h1 className='text-2xl font-bold tracking-tight'>History &amp; SQL</h1>
          <p className='text-muted-foreground text-sm'>
            DataFusion query surface over sites, equips, points, his, sparks
          </p>
        </div>

        <Card className='mb-4'>
          <CardContent className='space-y-3 p-4'>
            <Textarea
              value={sql}
              onChange={(e) => setSql(e.target.value)}
              rows={4}
              className='font-mono text-[12.5px]'
              spellCheck={false}
            />
            <div className='flex items-center gap-2'>
              <Button size='sm' onClick={() => run.mutate()} disabled={run.isPending}>
                <Play className='size-3.5' /> Run
              </Button>
              {run.isError && (
                <span className='text-sev-fault text-xs'>
                  {run.error instanceof ApiError ? run.error.message : 'Query failed'}
                </span>
              )}
            </div>
          </CardContent>
        </Card>

        {run.data && (
          <Card>
            <CardContent className='overflow-x-auto p-0'>
              <table className='w-full text-[12.5px]'>
                <thead>
                  <tr className='border-border border-b'>
                    {run.data.columns.map((c) => (
                      <th key={c} className='text-muted-foreground px-3 py-2 text-left font-medium'>
                        {c}
                      </th>
                    ))}
                  </tr>
                </thead>
                <tbody>
                  {run.data.rows.map((row, i) => (
                    <tr key={i} className='border-border/60 border-b last:border-0'>
                      {row.map((cell, j) => (
                        <td key={j} className='tabular px-3 py-1.5 font-mono'>
                          {formatValue(cell)}
                        </td>
                      ))}
                    </tr>
                  ))}
                </tbody>
              </table>
            </CardContent>
          </Card>
        )}
      </Main>
    </>
  )
}
