import { useMemo, useState } from 'react'
import { useBoards, useCreateWidget, useEquips, usePoints } from '@/api/hooks'
import { pointKeyexpr } from '@/api/keyexpr'
import type { CreateWidget, Site, Uuid } from '@/api/types'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import type { PaletteEntry } from '../lib/palette'

type BindablePalette = Extract<PaletteEntry, { available: true }>

type WidgetBinderProps = {
  site: Site
  entry: BindablePalette | null
  onClose: () => void
}

/**
 * Binder dialog: a point cascade (equip -> point) for `point_*` kinds or a
 * board slug for `board_output`, then `POST /widgets`. Targets come straight
 * from the live API — the point keyexpr is the same string the server stores.
 */
export function WidgetBinder({ site, entry, onClose }: WidgetBinderProps) {
  return (
    <Dialog open={entry !== null} onOpenChange={(o) => !o && onClose()}>
      <DialogContent className='sm:max-w-md'>
        {entry ? <BinderBody site={site} entry={entry} onClose={onClose} /> : null}
      </DialogContent>
    </Dialog>
  )
}

function BinderBody({
  site,
  entry,
  onClose,
}: {
  site: Site
  entry: BindablePalette
  onClose: () => void
}) {
  const create = useCreateWidget(site.id)
  const [title, setTitle] = useState('')
  const [target, setTarget] = useState('')
  const [error, setError] = useState<string | null>(null)

  const binder =
    entry.bind === 'point' ? (
      <PointBinder
        site={site}
        onPick={(keyexpr, suggested) => {
          setTarget(keyexpr)
          if (!title.trim()) setTitle(suggested)
        }}
      />
    ) : (
      <BoardBinder
        onPick={(slug, suggested) => {
          setTarget(slug)
          if (!title.trim()) setTitle(suggested)
        }}
      />
    )

  const submit = () => {
    if (!title.trim() || !target.trim()) {
      setError('Pick a target and give the tile a title.')
      return
    }
    const body: CreateWidget = {
      site_id: site.id,
      kind: entry.kind,
      title: title.trim(),
      target: target.trim(),
    }
    create.mutate(body, {
      onSuccess: onClose,
      onError: (e) => setError((e as Error).message),
    })
  }

  return (
    <>
      <DialogHeader>
        <DialogTitle className='flex items-center gap-2 text-[15px]'>
          <entry.icon className='text-primary size-4' /> Pin {entry.label}
        </DialogTitle>
        <DialogDescription>{entry.description}</DialogDescription>
      </DialogHeader>

      <div className='space-y-3 py-1'>
        {binder}
        <div className='space-y-1.5'>
          <Label htmlFor='widget-title' className='text-[12px]'>
            Tile title
          </Label>
          <Input
            id='widget-title'
            value={title}
            onChange={(e) => setTitle(e.target.value)}
            placeholder='Supply Air Temp'
          />
        </div>
        {error ? <p className='text-sev-fault text-[12px]'>{error}</p> : null}
      </div>

      <DialogFooter>
        <Button variant='ghost' onClick={onClose}>
          Cancel
        </Button>
        <Button onClick={submit} disabled={create.isPending || !target}>
          {create.isPending ? 'Pinning…' : 'Pin widget'}
        </Button>
      </DialogFooter>
    </>
  )
}

function PointBinder({
  site,
  onPick,
}: {
  site: Site
  onPick: (keyexpr: string, suggestedTitle: string) => void
}) {
  const { data: equips = [] } = useEquips(site.id)
  const [selectedEquipId, setSelectedEquipId] = useState<Uuid | undefined>()
  const equipId = selectedEquipId ?? equips[0]?.id
  const { data: points = [] } = usePoints({ equipId })
  const equip = useMemo(() => equips.find((e) => e.id === equipId), [equips, equipId])

  return (
    <div className='grid grid-cols-2 gap-2'>
      <div className='space-y-1.5'>
        <Label className='text-[12px]'>Equipment</Label>
        <Select value={equipId} onValueChange={setSelectedEquipId}>
          <SelectTrigger size='sm' className='w-full'>
            <SelectValue placeholder='Select equip' />
          </SelectTrigger>
          <SelectContent>
            {equips.map((e) => (
              <SelectItem key={e.id} value={e.id}>
                {e.display_name}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>
      <div className='space-y-1.5'>
        <Label className='text-[12px]'>Point</Label>
        <Select
          disabled={!equip}
          onValueChange={(id) => {
            const point = points.find((p) => p.id === id)
            if (equip && point) onPick(pointKeyexpr(site, equip, point), point.display_name)
          }}
        >
          <SelectTrigger size='sm' className='w-full'>
            <SelectValue placeholder='Select point' />
          </SelectTrigger>
          <SelectContent>
            {points.map((p) => (
              <SelectItem key={p.id} value={p.id}>
                {p.display_name}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>
    </div>
  )
}

function BoardBinder({
  onPick,
}: {
  onPick: (slug: string, suggestedTitle: string) => void
}) {
  const { data: boards = [] } = useBoards()
  return (
    <div className='space-y-1.5'>
      <Label className='text-[12px]'>Board</Label>
      <Select
        onValueChange={(slug) => {
          const board = boards.find((b) => b.slug === slug)
          if (board) onPick(board.slug, board.display_name)
        }}
      >
        <SelectTrigger size='sm' className='w-full'>
          <SelectValue placeholder='Select a board' />
        </SelectTrigger>
        <SelectContent>
          {boards.map((b) => (
            <SelectItem key={b.slug} value={b.slug}>
              {b.display_name}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </div>
  )
}
