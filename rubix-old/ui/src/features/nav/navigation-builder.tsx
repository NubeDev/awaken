/**
 * The Navigation builder (docs/design/page-context-and-nav.md §7): an admin
 * builds the nested nav — add a group, mount a board with a context, add a
 * static page, reorder/reparent, and grant access per node inline. One board can
 * be mounted at many nodes, each with its own `context.values`, which is the
 * fleet story this screen exists for. Gated on `whoami.can_admin`.
 */
import { ArrowDown, ArrowUp, Trash2 } from 'lucide-react'
import { useDashboards } from '@/api/hooks'
import type { Dashboard, NavNode } from '@/api/types'
import { useScope } from '@/context/scope-provider'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card } from '@/components/ui/card'
import { Main } from '@/components/layout/main'
import { PageHeader } from '@/components/layout/page-header'
import { AdminGuard } from '@/features/admin/admin-guard'
import {
  useCreateNavNode,
  useDeleteNavNode,
  useNavTree,
  usePatchNavNode,
} from './use-nav'
import { assembleNavTree, type NavTreeNode } from './tree'
import { movePatches } from './reorder'
import { AddNodeForm } from './add-node-form'
import { NodeGrants } from './node-grants'

export function NavigationBuilder() {
  return (
    <AdminGuard title='Navigation' sub='Build the nav tree and grant per-node access'>
      <Body />
    </AdminGuard>
  )
}

function Body() {
  const { org } = useScope()
  const { data: nodes = [] } = useNavTree(org)
  const { data: dashboards = [] } = useDashboards(org)
  const create = useCreateNavNode()
  const tree = assembleNavTree(nodes)

  return (
    <>
      <PageHeader
        title='Navigation'
        sub='One board, mounted at many nodes — each node parameterises and gates it'
      />
      <Main fluid fixed className='flex min-h-0 flex-col gap-3'>
        {org ? (
          <AddNodeForm
            org={org}
            nodes={nodes}
            dashboards={dashboards}
            onCreate={(body) => create.mutate(body)}
            pending={create.isPending}
          />
        ) : null}

        <div className='scroll min-h-0 flex-1 space-y-2 overflow-y-auto pe-1'>
          {tree.length === 0 ? (
            <Card className='grid h-32 place-items-center'>
              <p className='text-sm text-muted-foreground'>
                No nav nodes yet. Add a group, then mount boards under it.
              </p>
            </Card>
          ) : (
            tree.map((node) => (
              <NodeRow
                key={node.id}
                node={node}
                depth={0}
                org={org}
                nodes={nodes}
                dashboards={dashboards}
              />
            ))
          )}
        </div>
      </Main>
    </>
  )
}

function NodeRow({
  node,
  depth,
  org,
  nodes,
  dashboards,
}: {
  node: NavTreeNode
  depth: number
  org: string | undefined
  nodes: NavNode[]
  dashboards: Dashboard[]
}) {
  const patch = usePatchNavNode()
  const del = useDeleteNavNode()

  const move = (delta: -1 | 1) => {
    for (const p of movePatches(nodes, node.id, delta)) {
      patch.mutate({ id: p.id, body: { sort_order: p.sort_order } })
    }
  }

  return (
    <div style={{ marginInlineStart: depth * 16 }} className='space-y-2'>
      <Card className='space-y-2 p-2.5'>
        <div className='flex items-center gap-2'>
          <Badge variant='outline' className='font-normal'>
            {node.target.kind}
          </Badge>
          <span className='text-sm font-medium'>{node.title}</span>
          <span className='text-[11px] text-muted-foreground'>
            {describeTarget(node, dashboards)}
          </span>
          <div className='ms-auto flex items-center gap-1'>
            <Button
              size='icon'
              variant='ghost'
              className='size-7'
              onClick={() => move(-1)}
              title='Move up'
            >
              <ArrowUp className='size-3.5' />
            </Button>
            <Button
              size='icon'
              variant='ghost'
              className='size-7'
              onClick={() => move(1)}
              title='Move down'
            >
              <ArrowDown className='size-3.5' />
            </Button>
            <Button
              size='icon'
              variant='ghost'
              className='size-7 text-sev-fault'
              onClick={() => del.mutate(node.id)}
              title='Delete'
            >
              <Trash2 className='size-3.5' />
            </Button>
          </div>
        </div>
        {org ? <NodeGrants org={org} nodeId={node.id} /> : null}
      </Card>
      {node.children.map((child) => (
        <NodeRow
          key={child.id}
          node={child}
          depth={depth + 1}
          org={org}
          nodes={nodes}
          dashboards={dashboards}
        />
      ))}
    </div>
  )
}

/** A one-line human description of a node's target for the row. */
function describeTarget(node: NavNode, dashboards: Dashboard[]): string {
  if (node.target.kind === 'dashboard') {
    const dashboardId = node.target.dashboard_id
    const board = dashboards.find((d) => d.id === dashboardId)
    const values = node.context?.values
      ? Object.entries(node.context.values)
          .map(([k, v]) => `${k}=${String(v)}`)
          .join(', ')
      : ''
    return `${board ? board.slug : 'unknown board'}${values ? ` · ${values}` : ''}`
  }
  if (node.target.kind === 'route') return `→ ${node.target.route}`
  return ''
}
