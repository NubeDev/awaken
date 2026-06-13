/**
 * The nested, user-built navigation tree in the sidebar (docs/design/page-
 * context-and-nav.md §4): `group` nodes expand/collapse, `dashboard`/`route`
 * nodes are links. The tree is the server-filtered set (only nodes the caller
 * holds `view` on), so it renders exactly what the user may navigate — no
 * client-side gate. A dashboard link carries `?nav=<id>` so the opened board
 * binds to the node's context.
 */
import { Link, useLocation } from '@tanstack/react-router'
import { ChevronRight, Folder } from 'lucide-react'
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from '@/components/ui/collapsible'
import {
  SidebarGroup,
  SidebarGroupLabel,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarMenuSub,
} from '@/components/ui/sidebar'
import type { Dashboard } from '@/api/types'
import { useDashboards } from '@/api/hooks'
import { useScope } from '@/context/scope-provider'
import { useNavTree } from './use-nav'
import { assembleNavTree, type NavTreeNode } from './tree'
import { targetHref } from './target-href'

/** The sidebar's user-built nav tree. Renders nothing while the tree is empty
 *  (a fresh org gets a default tree seeded server-side), so the flat scope nav
 *  remains the sole nav until nodes exist. */
export function NavTree() {
  const { org, site } = useScope()
  const { data: nodes = [] } = useNavTree(org)
  const { data: dashboards = [] } = useDashboards(org)
  const href = useLocation({ select: (l) => l.href })

  if (nodes.length === 0) return null
  const tree = assembleNavTree(nodes)

  return (
    <SidebarGroup>
      <SidebarGroupLabel>Navigation</SidebarGroupLabel>
      <SidebarMenu>
        {tree.map((node) => (
          <NavTreeItem
            key={node.id}
            node={node}
            depth={0}
            org={org}
            siteSlug={site?.slug}
            dashboards={dashboards}
            activeHref={href}
          />
        ))}
      </SidebarMenu>
    </SidebarGroup>
  )
}

function NavTreeItem({
  node,
  depth,
  org,
  siteSlug,
  dashboards,
  activeHref,
}: {
  node: NavTreeNode
  depth: number
  org: string | undefined
  siteSlug: string | undefined
  dashboards: Dashboard[]
  activeHref: string
}) {
  const hasChildren = node.children.length > 0

  // A group (or any node with children) is collapsible; a leaf is a link.
  if (node.target.kind === 'group' || hasChildren) {
    return (
      <Collapsible defaultOpen className='group/collapsible'>
        <SidebarMenuItem>
          <CollapsibleTrigger asChild>
            <SidebarMenuButton tooltip={node.title}>
              <Folder />
              <span>{node.title}</span>
              {hasChildren ? (
                <ChevronRight className='ms-auto transition-transform duration-200 group-data-[state=open]/collapsible:rotate-90' />
              ) : null}
            </SidebarMenuButton>
          </CollapsibleTrigger>
          {hasChildren ? (
            <CollapsibleContent>
              <SidebarMenuSub>
                {node.children.map((child) => (
                  <NavTreeItem
                    key={child.id}
                    node={child}
                    depth={depth + 1}
                    org={org}
                    siteSlug={siteSlug}
                    dashboards={dashboards}
                    activeHref={activeHref}
                  />
                ))}
              </SidebarMenuSub>
            </CollapsibleContent>
          ) : null}
        </SidebarMenuItem>
      </Collapsible>
    )
  }

  const to = targetHref({
    target: node.target,
    nodeId: node.id,
    org,
    siteSlug,
    dashboards,
  })
  if (!to) {
    // A dashboard target whose board the caller cannot resolve: render a
    // disabled label rather than a dead link.
    return (
      <SidebarMenuItem>
        <SidebarMenuButton disabled tooltip={node.title}>
          <span>{node.title}</span>
        </SidebarMenuButton>
      </SidebarMenuItem>
    )
  }

  return (
    <SidebarMenuItem>
      <SidebarMenuButton
        asChild
        tooltip={node.title}
        isActive={activeHref === to}
      >
        <Link to={to}>
          <span>{node.title}</span>
        </Link>
      </SidebarMenuButton>
    </SidebarMenuItem>
  )
}
