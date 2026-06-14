import { useEffect } from 'react'
import { useRunStoredBoard } from '@/api/hooks'
import { useScope } from '@/context/scope-provider'
import type { Widget } from '@/api/types'
import { Card } from '@/components/ui/card'

const RUN_INTERVAL = 5_000

/**
 * Board-output tile: runs the stored flow (`POST /boards/{slug}/run`) and shows
 * its outport packets. There is no stored-output read endpoint, so the latest
 * output is produced by running the flow on the app's live interval. The flow is
 * resolved in the active org; the tile's `site_id` scopes a site flow.
 */
export function BoardOutputCard({ widget }: { widget: Widget }) {
  const { org } = useScope()
  const run = useRunStoredBoard()
  const { mutate } = run

  useEffect(() => {
    if (!org) return
    const fire = () =>
      mutate({ slug: widget.target, org, siteId: widget.site_id })
    fire()
    const id = setInterval(fire, RUN_INTERVAL)
    return () => clearInterval(id)
  }, [mutate, widget.target, widget.site_id, org])

  const outputs = run.data?.outputs ?? []

  return (
    <Card className='gap-2 p-3.5'>
      <div className='flex items-center justify-between'>
        <span className='eyebrow text-[10px]'>{widget.title}</span>
        <span className='font-mono text-[10px] text-muted-foreground'>
          {widget.target}
        </span>
      </div>
      {run.isError ? (
        <p className='text-[11.5px] text-sev-fault'>
          {(run.error as Error).message}
        </p>
      ) : outputs.length === 0 ? (
        <p className='text-[11.5px] text-muted-foreground'>
          {run.isPending ? 'Running…' : 'No outport packets.'}
        </p>
      ) : (
        <div className='space-y-1'>
          {outputs.map((o, i) => (
            <div
              key={`${o.node}-${o.port}-${i}`}
              className='rounded-md border border-border px-2 py-1.5 text-[11px]'
            >
              <div className='flex items-center justify-between font-mono'>
                <span className='truncate'>{o.node}</span>
                <span className='text-muted-foreground'>{o.port}</span>
              </div>
              <div className='mt-1 truncate font-mono text-muted-foreground'>
                {JSON.stringify(o.value)}
              </div>
            </div>
          ))}
        </div>
      )}
    </Card>
  )
}
