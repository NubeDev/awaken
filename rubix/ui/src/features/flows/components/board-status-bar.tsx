import { GitFork, History, Play } from 'lucide-react'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'

type BoardStatusBarProps = {
  name: string
  nodeCount: number
  edgeCount: number
}

/** Strip above the canvas: board identity, run state, and board actions. */
export function BoardStatusBar({ name, nodeCount, edgeCount }: BoardStatusBarProps) {
  return (
    <div className='flex items-center gap-3 pb-2.5'>
      <div className='min-w-0'>
        <div className='truncate text-[13.5px] font-semibold'>{name}</div>
        <div className='text-muted-foreground text-[11px]'>control board · v4</div>
      </div>
      <Badge variant='positive' className='gap-1.5'>
        <span className='bg-positive size-1.5 rounded-full' />
        deployed · running
      </Badge>
      <span className='text-muted-foreground text-[11.5px]'>
        {nodeCount} nodes · {edgeCount} wires
      </span>
      <div className='ms-auto flex items-center gap-2'>
        <Button variant='ghost' size='sm'>
          <Play className='size-3.5' /> Test run
        </Button>
        <Button variant='ghost' size='sm'>
          <History className='size-3.5' /> Versions
        </Button>
        <Button size='sm'>
          <GitFork className='size-3.5' /> Deploy
        </Button>
      </div>
    </div>
  )
}
