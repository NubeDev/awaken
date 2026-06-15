// Generic native pages (devices / data / rules / reports / settings). Where a
// page maps cleanly onto real records it renders them (devices = equipment);
// where the backend plane isn't wired yet it says so honestly rather than
// showing invented rows (CLAUDE.md: no placeholder/fake data).

import { getRouteApi } from '@tanstack/react-router'
import { Cpu, Database, FileBarChart, GitBranch, Settings } from 'lucide-react'
import { useRecords } from '../hooks/useRecords'
import { toEquips } from '../utils/derive'
import { TopBar } from '../components/ui/TopBar'
import { ErrorView, LoadingView, EmptyView } from '../components/ui/StateView'

const route = getRouteApi('/t/$tenant/$page')

const META: Record<string, { title: string; icon: typeof Cpu; sub: string }> = {
  devices: { title: 'Devices', icon: Cpu, sub: 'Equipment reporting in this tenant' },
  data: { title: 'Data Sources', icon: Database, sub: 'Registered connectors' },
  rules: { title: 'Rules', icon: GitBranch, sub: 'Automations' },
  reports: { title: 'Reports', icon: FileBarChart, sub: 'Scheduled' },
  settings: { title: 'Settings', icon: Settings, sub: 'Site & team' },
}

export function GenericPage() {
  const { tenant, page } = route.useParams()
  const { site } = route.useSearch()
  const { data: records, isLoading, error } = useRecords(tenant)
  const meta = META[page] ?? { title: page, icon: Settings, sub: '' }
  const Icon = meta.icon

  const equips = records ? toEquips(records).filter((e) => !site || e.site === site) : []

  return (
    <div className="h-full flex flex-col">
      <TopBar tenant={tenant} site={site} crumbs={[meta.title]} livePoints={records?.length} />
      <div className="flex-1 overflow-auto p-6">
        <div className="max-w-[760px] mx-auto">
          <div className="flex items-center gap-3">
            <div className="size-11 rounded-xl bg-panel2 border border-border grid place-items-center">
              <Icon size={20} className="text-muted" />
            </div>
            <div>
              <h1 className="text-[22px] font-semibold tracking-tight">{meta.title}</h1>
              <div className="text-[13px] text-muted">{meta.sub}</div>
            </div>
          </div>

          {isLoading && <LoadingView />}
          {error && <ErrorView error={error} />}

          {records && page === 'devices' && (
            <div className="mt-5 rounded-2xl border border-border bg-panel2 divide-y divide-border overflow-hidden">
              {equips.length === 0 && <div className="px-4 py-8 text-center text-[13px] text-muted">No equipment for this site.</div>}
              {equips.map((e) => (
                <div key={e.id} className="flex items-center gap-3 px-4 py-3.5">
                  <span className="size-1.5 rounded-full bg-green" />
                  <span className="text-[13.5px] flex-1">{e.name}</span>
                  <span className="text-[12px] text-muted mono">
                    {e.domain} · {e.type}
                  </span>
                </div>
              ))}
            </div>
          )}

          {records && page !== 'devices' && (
            <div className="mt-5">
              <EmptyView
                title={`${meta.title} isn’t wired to a backend plane yet`}
                hint="This screen will read live data once the backend surface lands. Use Admin · Records to browse the raw store today."
              />
            </div>
          )}
        </div>
      </div>
    </div>
  )
}
