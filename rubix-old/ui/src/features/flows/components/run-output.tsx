import type { RunBoardResponse } from '@/api/types'

/**
 * Last test-run result: every outport packet the engine produced, in node
 * order. Real engine output — never synthesized. Hidden until a run has
 * happened so the inspector keeps its default empty state.
 */
export function RunOutput({ result }: { result: RunBoardResponse | undefined }) {
  if (!result) return null
  return (
    <div className='border-border border-t pt-3'>
      <div className='eyebrow pb-1.5 text-[9.5px]'>Last test run · {result.outputs.length} outputs</div>
      {result.outputs.length === 0 ? (
        <div className='text-muted-foreground text-[11.5px]'>
          The board settled without emitting any outport packets. A trigger only
          fires once its period elapses — run again after that to see its output.
        </div>
      ) : (
        <div className='space-y-1'>
          {result.outputs.map((o, i) => (
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
    </div>
  )
}
