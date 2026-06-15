/** Page heading shown inside the top chrome bar: title · live dot · subtitle. */
export function PageTitle({ title, sub }: { title: string; sub?: string }) {
  return (
    <div className='min-w-0'>
      <div className='flex items-center gap-2'>
        <h1 className='truncate text-[15px] leading-none font-semibold tracking-tight'>{title}</h1>
        <span className='live-dot shrink-0' aria-label='live' />
      </div>
      {sub ? (
        <p className='text-muted-foreground mt-1 truncate text-[11.5px] leading-none'>{sub}</p>
      ) : null}
    </div>
  )
}
