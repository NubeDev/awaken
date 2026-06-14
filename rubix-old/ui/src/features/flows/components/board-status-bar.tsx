import { useState } from 'react'
import { History, Loader2, Pencil, Play, Save, Trash2 } from 'lucide-react'
import { useDeleteBoard } from '@/api/hooks'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from '@/components/ui/tooltip'
import type { BoardView } from '@/api/types'
import { ConfirmDialog } from '@/components/confirm-dialog'
import { BoardEditDialog } from './board-edit-dialog'

type BoardStatusBarProps = {
  /** The stored board — identity, trigger, and graph for edit/delete. */
  board: BoardView
  /** Board slug — identity for metadata patch and delete. */
  slug: string
  name: string
  version: number
  enabled: boolean
  /** Trigger interval in seconds when the board runs continuously; else undefined. */
  intervalSeconds?: number
  nodeCount: number
  edgeCount: number
  running: boolean
  onTestRun: () => void
  /** Unsaved canvas edits exist. */
  dirty: boolean
  /** A save is in flight. */
  saving: boolean
  /** Persist the current canvas as a new board version. */
  onSave: () => void
  /** Called after the board is deleted, so the editor can drop its selection. */
  onDeleted?: () => void
}

/**
 * Strip above the canvas: board identity and real status. Every value comes
 * from the stored board — version, enabled flag, and node/wire counts. There is
 * no run-state source on the wire today, so no "running" claim is made. Deploy
 * and Versions are honest disabled controls (publishing lands with the editor).
 */
export function BoardStatusBar({
  board,
  slug,
  name,
  version,
  enabled,
  intervalSeconds,
  nodeCount,
  edgeCount,
  running,
  onTestRun,
  dirty,
  saving,
  onSave,
  onDeleted,
}: BoardStatusBarProps) {
  const del = useDeleteBoard()
  const [editOpen, setEditOpen] = useState(false)
  const [confirmOpen, setConfirmOpen] = useState(false)
  return (
    <div className='flex items-center gap-3 pb-2.5'>
      <div className='min-w-0'>
        <div className='truncate text-[13.5px] font-semibold'>{name}</div>
        <div className='text-[11px] text-muted-foreground'>
          control board · v{version}
        </div>
      </div>
      {enabled && intervalSeconds ? (
        <Badge variant='positive' className='gap-1.5'>
          <span className='size-1.5 animate-pulse rounded-full bg-positive' />
          live · every {intervalSeconds}s
        </Badge>
      ) : enabled ? (
        <Badge variant='muted' className='gap-1.5'>
          <span className='size-1.5 rounded-full bg-muted-foreground' />
          on demand
        </Badge>
      ) : (
        <Badge variant='muted' className='gap-1.5'>
          <span className='size-1.5 rounded-full bg-muted-foreground' />
          disabled
        </Badge>
      )}
      <span className='text-[11.5px] text-muted-foreground'>
        {nodeCount} nodes · {edgeCount} wires
        {dirty && <span className='ms-1.5 text-primary'>· unsaved</span>}
      </span>
      <div className='ms-auto flex items-center gap-2'>
        <Button
          variant='ghost'
          size='sm'
          onClick={onTestRun}
          disabled={running}
        >
          {running ? (
            <Loader2 className='size-3.5 animate-spin' />
          ) : (
            <Play className='size-3.5' />
          )}
          Test run
        </Button>
        <Button variant='ghost' size='sm' onClick={() => setEditOpen(true)}>
          <Pencil className='size-3.5' /> Edit
        </Button>
        <DisabledAction
          label='Versions'
          icon={<History className='size-3.5' />}
        />
        <Button
          variant='default'
          size='sm'
          onClick={onSave}
          disabled={!dirty || saving}
        >
          {saving ? (
            <Loader2 className='size-3.5 animate-spin' />
          ) : (
            <Save className='size-3.5' />
          )}
          Save version
        </Button>
        <Button
          variant='ghost'
          size='icon'
          className='size-8 text-sev-fault'
          title='Delete board'
          onClick={() => setConfirmOpen(true)}
        >
          <Trash2 className='size-3.5' />
        </Button>
      </div>

      <BoardEditDialog board={board} open={editOpen} onOpenChange={setEditOpen} />
      <ConfirmDialog
        open={confirmOpen}
        onOpenChange={setConfirmOpen}
        destructive
        title={`Delete board ${name}?`}
        desc={
          <>
            This deletes <strong>every version</strong> of <code>{slug}</code>.
            A running scheduler keeps the old version until its next launch.
            This cannot be undone.
          </>
        }
        confirmText='Delete board'
        isLoading={del.isPending}
        handleConfirm={() =>
          del.mutate(
            { slug, org: board.org, siteId: board.site_id ?? undefined },
            {
              onSuccess: () => {
                setConfirmOpen(false)
                onDeleted?.()
              },
            }
          )
        }
      />
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
