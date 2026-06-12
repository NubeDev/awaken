import { GitFork, History, Loader2, Play } from 'lucide-react'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip'

type BoardStatusBarProps = {
  name: string
  version: number
  enabled: boolean
  nodeCount: number
  edgeCount: number
  running: boolean
  onTestRun: () => void
}

/**
 * Strip above the canvas: board identity and real status. Every value comes
 * from the stored board — version, enabled flag, and node/wire counts. There is
 * no run-state source on the wire today, so no "running" claim is made. Deploy
 * and Versions are honest disabled controls (publishing lands with the editor).
 */
export function BoardStatusBar({
  name,
  version,
  enabled,
  nodeCount,
  edgeCount,
  running,
  onTestRun,
}: BoardStatusBarProps) {
  return (
    <div className='flex items-center gap-3 pb-2.5'>
      <div className='min-w-0'>
        <div className='truncate text-[13.5px] font-semibold'>{name}</div>
        <div className='text-muted-foreground text-[11px]'>control board · v{version}</div>
      </div>
      {enabled ? (
        <Badge variant='positive' className='gap-1.5'>
          <span className='bg-positive size-1.5 rounded-full' />
          enabled
        </Badge>
      ) : (
        <Badge variant='muted' className='gap-1.5'>
          <span className='bg-muted-foreground size-1.5 rounded-full' />
          disabled
        </Badge>
      )}
      <span className='text-muted-foreground text-[11.5px]'>
        {nodeCount} nodes · {edgeCount} wires
      </span>
      <div className='ms-auto flex items-center gap-2'>
        <Button variant='ghost' size='sm' onClick={onTestRun} disabled={running}>
          {running ? <Loader2 className='size-3.5 animate-spin' /> : <Play className='size-3.5' />}
          Test run
        </Button>
        <DisabledAction label='Versions' icon={<History className='size-3.5' />} />
        <DisabledAction label='Deploy' icon={<GitFork className='size-3.5' />} primary />
      </div>
    </div>
  )
}

/** An action that is intentionally inert until the board editor ships. */
function DisabledAction({
  label,
  icon,
  primary,
}: {
  label: string
  icon: React.ReactNode
  primary?: boolean
}) {
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        {/* span wrapper: a disabled button does not emit pointer events for the tooltip. */}
        <span className='inline-flex'>
          <Button variant={primary ? 'default' : 'ghost'} size='sm' disabled>
            {icon} {label}
          </Button>
        </span>
      </TooltipTrigger>
      <TooltipContent>publishing lands with the editor</TooltipContent>
    </Tooltip>
  )
}
