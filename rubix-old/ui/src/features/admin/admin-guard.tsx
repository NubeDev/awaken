import type { ReactNode } from 'react'
import { ShieldAlert } from 'lucide-react'
import { useWhoami } from '@/api/hooks'
import { Card } from '@/components/ui/card'
import { Main } from '@/components/layout/main'
import { PageHeader } from '@/components/layout/page-header'

/**
 * Gate the RBAC management surfaces (Members / Teams / Access) on the caller's
 * `can_admin` capability from `whoami`. A non-admin sees a clear "not
 * authorized" panel rather than empty tables of 403s. On the open dev server
 * `can_admin` is true, so the surfaces render (their mutations still require a
 * real admin server-side).
 */
export function AdminGuard({
  title,
  sub,
  children,
}: {
  title: string
  sub: string
  children: ReactNode
}) {
  const { data: whoami, isLoading } = useWhoami()

  if (isLoading) return null

  if (!whoami?.can_admin) {
    return (
      <>
        <PageHeader title={title} sub={sub} />
        <Main fluid>
          <Card className='grid h-40 place-items-center'>
            <div className='flex flex-col items-center gap-2 text-muted-foreground'>
              <ShieldAlert className='size-6' />
              <p className='text-sm'>
                You need admin access to manage this org.
              </p>
            </div>
          </Card>
        </Main>
      </>
    )
  }

  return <>{children}</>
}
