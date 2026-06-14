import { useMemo, useState } from 'react'
import {
  useBoards,
  useCreateWidget,
  useEquips,
  usePatchWidget,
  usePoints,
} from '@/api/hooks'
import { pointKeyexpr } from '@/api/keyexpr'
import { useScope } from '@/context/scope-provider'
import type { ChartType, CreateWidget, Site, Uuid } from '@/api/types'
import { cn } from '@/lib/utils'
import { CHART_TYPES } from '../lib/chart-types'
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
  /** Dashboard the tile pins onto. */
  dashboardId: Uuid
  entry: BindablePalette | null
  onClose: () => void
  /** When set (org-overview dashboard), the binder shows a site picker so the
   *  operator can pin tiles from any of the org's sites. Omit for a site board. */
  sites?: Site[]
  onSiteChange?: (siteId: string) => void
}

/**
 * Binder dialog: a point cascade (equip -> point) for `point_*` kinds or a
 * board slug for `board_output`, then `POST /widgets`. Targets come straight
 * from the live API — the point keyexpr is the same string the server stores.
 */
export function WidgetBinder({
  site,
  dashboardId,
  entry,
  onClose,
  sites,
  onSiteChange,
}: WidgetBinderProps) {
  return (
    <Dialog open={entry !== null} onOpenChange={(o) => !o && onClose()}>
      <DialogContent className='sm:max-w-md'>
        {entry ? (
          <BinderBody
            site={site}
            dashboardId={dashboardId}
            entry={entry}
            onClose={onClose}
            sites={sites}
            onSiteChange={onSiteChange}
          />
        ) : null}
      </DialogContent>
    </Dialog>
  )
}

function BinderBody({
  site,
  dashboardId,
  entry,
  onClose,
  sites,
  onSiteChange,
}: {
  site: Site
  dashboardId: Uuid
  entry: BindablePalette
  onClose: () => void
  sites?: Site[]
  onSiteChange?: (siteId: string) => void
}) {
  const create = useCreateWidget()
  const patch = usePatchWidget()
  const [title, setTitle] = useState('')
  const [target, setTarget] = useState('')
  const [chart, setChart] = useState<ChartType>('area')
  const [error, setError] = useState<string | null>(null)
  // A chart type only applies to the time-series tile; other kinds ignore it.
  const picksChart = entry.kind === 'point_history'

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
      dashboard_id: dashboardId,
      site_id: site.id,
      kind: entry.kind,
      title: title.trim(),
      target: target.trim(),
    }
    create.mutate(body, {
      // A non-default chart type is stored as the new tile's config; the canvas
      // auto-flows its layout until the operator drags it.
      onSuccess: (created) => {
        if (picksChart && chart !== 'area') {
          patch.mutate({ id: created.id, body: { settings: { config: { type: chart } } } })
        }
        onClose()
      },
      onError: (e) => setError((e as Error).message),
    })
  }

  return (
    <>
      <DialogHeader>
        <DialogTitle className='flex items-center gap-2 text-[15px]'>
          <entry.icon className='size-4 text-primary' /> Pin {entry.label}
        </DialogTitle>
        <DialogDescription>{entry.description}</DialogDescription>
      </DialogHeader>

      <div className='space-y-3 py-1'>
        {sites && sites.length > 0 && onSiteChange ? (
          <div className='space-y-1.5'>
            <Label className='text-[12px]'>Site (this overview spans sites)</Label>
            <Select value={site.id} onValueChange={onSiteChange}>
              <SelectTrigger size='sm' className='w-full'>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {sites.map((s) => (
                  <SelectItem key={s.id} value={s.id}>
                    {s.display_name}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
        ) : null}
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
        {picksChart ? (
          <div className='space-y-1.5'>
            <Label className='text-[12px]'>Chart type</Label>
            <div className='grid grid-cols-4 gap-1.5'>
              {CHART_TYPES.map((c) => (
                <button
                  key={c.type}
                  type='button'
                  onClick={() => setChart(c.type)}
                  className={cn(
                    'flex flex-col items-center gap-1 rounded-md border px-2 py-2 text-[11px] transition-colors',
                    chart === c.type
                      ? 'border-primary bg-primary/10 text-foreground'
                      : 'border-border text-muted-foreground hover:bg-accent'
                  )}
                >
                  <c.icon className='size-4' />
                  {c.label}
                </button>
              ))}
            </div>
          </div>
        ) : null}
        {error ? <p className='text-[12px] text-sev-fault'>{error}</p> : null}
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
  const equip = useMemo(
    () => equips.find((e) => e.id === equipId),
    [equips, equipId]
  )

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
            if (equip && point)
              onPick(pointKeyexpr(site, equip, point), point.display_name)
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
  const { org, site } = useScope()
  const { data: boards = [] } = useBoards(org, site?.id)
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
