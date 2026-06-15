// Admin · Records browser — the embedded admin console, scoped to the same
// tenant the operator is in (PRODUCT-UI "Admin console — embedded, not
// separate"). Reuses <RecordGrid> over the generic /records surface.

import { getRouteApi } from '@tanstack/react-router'
import { Table2 } from 'lucide-react'
import { useRecords } from '../hooks/useRecords'
import { TopBar } from '../components/ui/TopBar'
import { RecordGrid } from '../components/data/RecordGrid'
import { ErrorView, LoadingView } from '../components/ui/StateView'

const route = getRouteApi('/t/$tenant/admin/records')

export function AdminRecords() {
  const { tenant } = route.useParams()
  const { site } = route.useSearch()
  const { data: records, isLoading, error } = useRecords(tenant)

  return (
    <div className="h-full flex flex-col">
      <TopBar tenant={tenant} site={site} crumbs={['Admin', 'Records']} livePoints={records?.length} />
      <div className="flex-1 overflow-auto p-6">
        <div className="max-w-[1080px] mx-auto">
          <div className="flex items-center gap-3 mb-5">
            <div className="size-11 rounded-xl bg-panel2 border border-border grid place-items-center">
              <Table2 size={20} className="text-muted" />
            </div>
            <div>
              <h1 className="text-[22px] font-semibold tracking-tight">Records</h1>
              <div className="text-[13px] text-muted">Every record this principal can read in tenant {tenant}.</div>
            </div>
          </div>
          {isLoading && <LoadingView label="Loading records…" />}
          {error && <ErrorView error={error} />}
          {records && <RecordGrid records={records} />}
        </div>
      </div>
    </div>
  )
}
