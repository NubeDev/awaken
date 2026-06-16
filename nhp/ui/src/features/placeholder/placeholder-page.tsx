import { Main } from '@/components/layout/main'

/**
 * App-shell placeholder for a feature a later WS owns. The header (with the
 * sidebar trigger + chrome) is rendered once by AuthenticatedLayout, so this
 * only renders the body. See WS-01.md.
 */
export function PlaceholderPage({
  title,
  owner,
}: {
  title: string
  owner: string
}) {
  return (
    <Main>
      <div className='flex h-full flex-col items-center justify-center gap-2 text-center'>
        <h2 className='text-2xl font-semibold tracking-tight'>{title}</h2>
        <p className='text-muted-foreground max-w-md text-sm'>
          This area is part of the NHP app shell. It will be built by {owner}.
        </p>
      </div>
    </Main>
  )
}
