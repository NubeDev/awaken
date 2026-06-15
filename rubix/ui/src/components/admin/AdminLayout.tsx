// The admin console shell — a left nav over the four developer surfaces, with the
// product TopBar on top. The console is the PocketBase-style admin over the
// generic API (ADMIN-UI.md): every screen here reads the substrate (records,
// principals, query) and knows NO domain type. Nav links are tenant-scoped.

import { Link, getRouteApi } from '@tanstack/react-router'
import { Bot, Database, KeyRound, LayoutDashboard, Table2, TerminalSquare, type LucideIcon } from 'lucide-react'
import type { ReactNode } from 'react'
import { TopBar } from '../ui/TopBar'
import { cn } from '@/lib/cn'

const route = getRouteApi('/t/$tenant/admin')

type AdminTab = 'schema' | 'records' | 'principals' | 'agents' | 'query' | 'dashboards'

interface NavItem {
  to: AdminTab
  path:
    | '/t/$tenant/admin/schema'
    | '/t/$tenant/admin/records'
    | '/t/$tenant/admin/principals'
    | '/t/$tenant/admin/agents'
    | '/t/$tenant/admin/query'
    | '/t/$tenant/admin/dashboards'
  label: string
  icon: LucideIcon
  sub: string
}

// Substrate surfaces only — no domain vocabulary. "Schema" inspects how the
// backend is shaped; the rest are generic CRUD/query over the gate. "Agents"
// provisions AI agents as scoped principals (AGENT.md).
const NAV: NavItem[] = [
  { to: 'schema', path: '/t/$tenant/admin/schema', label: 'Schema', icon: Database, sub: 'Kinds, fields & tags' },
  { to: 'records', path: '/t/$tenant/admin/records', label: 'Records', icon: Table2, sub: 'Browse & edit any record' },
  { to: 'principals', path: '/t/$tenant/admin/principals', label: 'Principals', icon: KeyRound, sub: 'Identities & grants' },
  { to: 'agents', path: '/t/$tenant/admin/agents', label: 'Agents', icon: Bot, sub: 'AI agents & tiers' },
  { to: 'query', path: '/t/$tenant/admin/query', label: 'Query', icon: TerminalSquare, sub: 'Run a query' },
  { to: 'dashboards', path: '/t/$tenant/admin/dashboards', label: 'Dashboards', icon: LayoutDashboard, sub: 'Pinned chart boards' },
]

export function AdminLayout({ active, children }: { active: string; children: ReactNode }) {
  const { tenant } = route.useParams()
  return (
    <div className="flex h-full flex-col">
      <TopBar tenant={tenant} crumbs={['Admin']} />
      <div className="flex min-h-0 flex-1">
        <nav className="w-60 shrink-0 border-r border-border p-3">
          <div className="px-2 pb-3 pt-1 text-[11px] font-medium uppercase tracking-wide text-muted-foreground">
            Console
          </div>
          <ul className="flex flex-col gap-1">
            {NAV.map((item) => {
              const Icon = item.icon
              const isActive = item.to === active
              return (
                <li key={item.to}>
                  <Link
                    to={item.path}
                    params={{ tenant }}
                    className={cn(
                      'flex items-start gap-2.5 rounded-lg px-2.5 py-2 text-sm transition-colors',
                      isActive
                        ? 'bg-primary/15 text-foreground'
                        : 'text-muted-foreground hover:bg-muted hover:text-foreground',
                    )}
                  >
                    <Icon size={16} className="mt-0.5 shrink-0" />
                    <span className="flex flex-col">
                      <span className="font-medium leading-tight">{item.label}</span>
                      <span className="text-[11px] text-muted-foreground">{item.sub}</span>
                    </span>
                  </Link>
                </li>
              )
            })}
          </ul>
        </nav>
        <main className="min-w-0 flex-1 overflow-auto p-6">{children}</main>
      </div>
    </div>
  )
}
