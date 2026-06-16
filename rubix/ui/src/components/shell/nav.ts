// The single source of truth for the sidebar nav — operator surfaces and the
// admin console, both tenant-scoped. Replaces the old per-layout NAV arrays (the
// admin one lived inside AdminLayout). Active state is derived from the matched
// route, not threaded as a prop. `to`/`params` are TanStack Router link targets.

import {
  Building2,
  Sparkles,
  Cpu,
  Database as DatabaseIcon,
  GitBranch,
  FileBarChart,
  Settings,
  Table2,
  KeyRound,
  Bot,
  TerminalSquare,
  LayoutDashboard,
  LayoutGrid,
  type LucideIcon,
} from 'lucide-react'

export interface NavItem {
  label: string
  /** TanStack Router `to` target (tenant param filled by the sidebar). */
  to: string
  /** Extra params beyond `tenant` (e.g. the `$page` slug). */
  params?: Record<string, string>
  icon: LucideIcon
}

export interface NavGroup {
  title: string
  items: NavItem[]
}

export const NAV: NavGroup[] = [
  {
    title: 'Operate',
    items: [
      { label: 'Home', to: '/t/$tenant', icon: LayoutGrid },
      { label: 'Building & Zones', to: '/t/$tenant/building', icon: Building2 },
      { label: 'Ask Rubix', to: '/t/$tenant/copilot', icon: Sparkles },
      { label: 'Devices', to: '/t/$tenant/$page', params: { page: 'devices' }, icon: Cpu },
      { label: 'Data Sources', to: '/t/$tenant/$page', params: { page: 'data' }, icon: DatabaseIcon },
      { label: 'Rules', to: '/t/$tenant/$page', params: { page: 'rules' }, icon: GitBranch },
      { label: 'Reports', to: '/t/$tenant/$page', params: { page: 'reports' }, icon: FileBarChart },
      { label: 'Settings', to: '/t/$tenant/$page', params: { page: 'settings' }, icon: Settings },
    ],
  },
  {
    title: 'Admin Console',
    items: [
      { label: 'Schema', to: '/t/$tenant/admin/schema', icon: DatabaseIcon },
      { label: 'Records', to: '/t/$tenant/admin/records', icon: Table2 },
      { label: 'Principals', to: '/t/$tenant/admin/principals', icon: KeyRound },
      { label: 'Agents', to: '/t/$tenant/admin/agents', icon: Bot },
      { label: 'Query', to: '/t/$tenant/admin/query', icon: TerminalSquare },
      { label: 'Rules', to: '/t/$tenant/admin/rules', icon: GitBranch },
      { label: 'Dashboards', to: '/t/$tenant/admin/dashboards', icon: LayoutDashboard },
    ],
  },
]
