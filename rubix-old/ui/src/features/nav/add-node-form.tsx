/**
 * The add-node form for the navigation builder (docs/design/page-context-and-
 * nav.md §4,§7): pick a target kind (group / dashboard mount / static route),
 * a parent, and — for a dashboard — the board plus a `context.values` override
 * (the per-mount parameterisation that lets one board serve a fleet). Context
 * values bind as SQL parameters downstream; the form never concatenates them.
 */
import { useState } from 'react'
import { Plus } from 'lucide-react'
import type {
  CreateNavNode,
  Dashboard,
  NavNode,
  NavRoute,
  NavTarget,
} from '@/api/types'
import { Button } from '@/components/ui/button'
import { Card } from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { parseValues } from './parse-values'

type TargetKind = NavTarget['kind']

const ROUTES: NavRoute[] = [
  'sites',
  'equips',
  'points',
  'dashboards',
  'datasources',
  'rules',
  'boards',
  'sparks',
  'runs',
  'audit',
  'access',
]

export function AddNodeForm({
  org,
  nodes,
  dashboards,
  onCreate,
  pending,
}: {
  org: string
  nodes: NavNode[]
  dashboards: Dashboard[]
  onCreate: (body: CreateNavNode) => void
  pending: boolean
}) {
  const [kind, setKind] = useState<TargetKind>('group')
  const [title, setTitle] = useState('')
  const [parentId, setParentId] = useState<string>('__root')
  const [dashboardId, setDashboardId] = useState('')
  const [route, setRoute] = useState<NavRoute>('datasources')
  // A `key=value` line per context override, parsed on submit.
  const [valuesText, setValuesText] = useState('')

  const reset = () => {
    setTitle('')
    setDashboardId('')
    setValuesText('')
  }

  const submit = () => {
    if (title.trim() === '') return
    let target: NavTarget
    if (kind === 'group') target = { kind: 'group' }
    else if (kind === 'route') target = { kind: 'route', route }
    else {
      if (!dashboardId) return
      target = { kind: 'dashboard', dashboard_id: dashboardId }
    }
    const body: CreateNavNode = {
      org,
      title: title.trim(),
      parent_id: parentId === '__root' ? null : parentId,
      target,
    }
    if (kind === 'dashboard') {
      const values = parseValues(valuesText)
      if (Object.keys(values).length > 0) body.context = { values }
    }
    onCreate(body)
    reset()
  }

  return (
    <Card className='space-y-2 p-3'>
      <p className='text-sm font-medium'>Add a node</p>
      <div className='flex flex-wrap items-end gap-2'>
        <div className='space-y-1'>
          <Label className='text-[11px]'>Kind</Label>
          <Select value={kind} onValueChange={(v) => setKind(v as TargetKind)}>
            <SelectTrigger size='sm' className='w-28'>
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value='group'>Group</SelectItem>
              <SelectItem value='dashboard'>Dashboard</SelectItem>
              <SelectItem value='route'>Static page</SelectItem>
            </SelectContent>
          </Select>
        </div>
        <div className='min-w-40 flex-1 space-y-1'>
          <Label className='text-[11px]'>Title</Label>
          <Input
            className='h-8'
            value={title}
            onChange={(e) => setTitle(e.target.value)}
            placeholder='e.g. Building-1'
          />
        </div>
        <div className='space-y-1'>
          <Label className='text-[11px]'>Parent</Label>
          <Select value={parentId} onValueChange={setParentId}>
            <SelectTrigger size='sm' className='w-40'>
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value='__root'>(root)</SelectItem>
              {nodes
                .filter((n) => n.target.kind === 'group')
                .map((n) => (
                  <SelectItem key={n.id} value={n.id}>
                    {n.title}
                  </SelectItem>
                ))}
            </SelectContent>
          </Select>
        </div>
      </div>

      {kind === 'dashboard' ? (
        <div className='flex flex-wrap items-end gap-2'>
          <div className='min-w-48 space-y-1'>
            <Label className='text-[11px]'>Board</Label>
            <Select value={dashboardId} onValueChange={setDashboardId}>
              <SelectTrigger size='sm' className='w-56'>
                <SelectValue placeholder='Pick a board…' />
              </SelectTrigger>
              <SelectContent>
                {dashboards.length === 0 ? (
                  <SelectItem value='__none' disabled>
                    No boards in this org
                  </SelectItem>
                ) : (
                  dashboards.map((d) => (
                    <SelectItem key={d.id} value={d.id}>
                      {d.title} ({d.slug})
                    </SelectItem>
                  ))
                )}
              </SelectContent>
            </Select>
          </div>
          <div className='min-w-48 flex-1 space-y-1'>
            <Label className='text-[11px]'>
              Context values (one <span className='font-mono'>key=value</span> per line)
            </Label>
            <textarea
              className='min-h-16 w-full rounded-md border bg-transparent px-2 py-1 font-mono text-[12px]'
              value={valuesText}
              onChange={(e) => setValuesText(e.target.value)}
              placeholder='site=s1'
            />
          </div>
        </div>
      ) : null}

      {kind === 'route' ? (
        <div className='space-y-1'>
          <Label className='text-[11px]'>Page</Label>
          <Select value={route} onValueChange={(v) => setRoute(v as NavRoute)}>
            <SelectTrigger size='sm' className='w-40'>
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {ROUTES.map((r) => (
                <SelectItem key={r} value={r}>
                  {r}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
      ) : null}

      <div className='flex justify-end'>
        <Button size='sm' onClick={submit} disabled={pending}>
          <Plus className='size-3.5' /> Add node
        </Button>
      </div>
    </Card>
  )
}
