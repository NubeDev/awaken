/**
 * The Empty state (DASHBOARDS-SCOPE §8: "never fabricate rows — empty → <Empty>,
 * not a fake zero"). A widget with no data renders this, never a zero point.
 */
export function Empty({ message = 'No data' }: { message?: string }) {
  return (
    <div className='text-muted-foreground flex h-full min-h-24 items-center justify-center text-sm'>
      {message}
    </div>
  )
}
