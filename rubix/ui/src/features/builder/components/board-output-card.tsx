import { useEffect } from 'react'
import { useRunStoredBoard } from '@/api/hooks'
import type { Widget } from '@/api/types'
import { Card } from '@/components/ui/card'

const RUN_INTERVAL = 5_000

/**
 * Board-output tile: runs the stored board (`POST /boards/{slug}/run`) and shows
 * its outport packets. There is no stored-output read endpoint, so the latest
 * output is produced by running the board on the app's live interval — the same
 * real engine output the flows page renders, never synthesized.
 */
export function BoardOutputCard({ widget }: { widget: Widget }) {
  const run = useRunStoredBoard()
  const { mutate } = run

  useEffect(() => {
    mutate(widget.target)
    const id = setInterval(() => mutate(widget.target), RUN_INTERVAL)
    return () => clearInterval(id)
  }, [mutate, widget.target])

  const outputs = run.data?.outputs ?? []

  return (
    <Card className='gap-2 p-3.5'>
      <div className='flex items-center justify-between'>
        <span className='eyebrow text-[10px]'>{widget.title}</span>
        <span className='text-muted-foreground font-mono text-[10px]'>{widget.target}</span>
      </div>
      {run.isError ? (
        <p className='text-sev-fault text-[11.5px]'>{(run.error as Error).message}</p>
      ) : outputs.length === 0 ? (
        <p className='text-muted-foreground text-[11.5px]'>
          {run.isPending ? 'Running…' : 'No outport packets.'}
        </p>
      ) : (
        <div className='space-y-1'>
          {outputs.map((o, i) => (
            <div
              key={`${o.node}-${o.port}-${i}`}
              className='border-border rounded-md border px-2 py-1.5 text-[11px]'
            >
              <div className='flex items-center justify-between font-mono'>
                <span className='truncate'>{o.node}</span>
                <span className='text-muted-foreground'>{o.port}</span>
              </div>
              <div className='text-muted-foreground mt-1 truncate font-mono'>
                {JSON.stringify(o.value)}
              </div>
            </div>
          ))}
        </div>
      )}
    </Card>
  )
}
