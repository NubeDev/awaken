import { useMemo, useState } from 'react'
import { SlidersHorizontal } from 'lucide-react'
import { useDashboards, useEquips, usePoints, useWidgets } from '@/api/hooks'
import { keyexprIndex, keyexprIndexMulti } from '@/api/keyexpr'
import type { Dashboard, Site, Widget } from '@/api/types'
import { useScope } from '@/context/scope-provider'
import { Button } from '@/components/ui/button'
import { Card } from '@/components/ui/card'
import { Main } from '@/components/layout/main'
import { PageHeader } from '@/components/layout/page-header'
import { TimeRangePicker } from '../time/time-range-picker'
import { useTimeRangeSync } from '../time/use-time-range'
import { useVariableResolution } from '../variables/use-resolution'
import { useVarUrlState } from '../variables/use-var-url-state'
import { usePageContext } from '../nav/use-page-context'
import { TagEditor } from '../nav/tag-editor'
import { HistoryButton } from '../audit/components/history-button'
import { VariableBar } from '../variables/variable-bar'
import { VariableEditorDialog } from '../variables/variable-editor-dialog'
import { DashboardFormDialog } from './components/dashboard-form-dialog'
import { DashboardPicker } from './components/dashboard-picker'
import { WidgetBinder } from './components/widget-binder'
import { WidgetCanvas } from './components/widget-canvas'
import { WidgetPalette } from './components/widget-palette'
import type { PaletteEntry } from './lib/palette'

type Bindable = Extract<PaletteEntry, { available: true }>

/**
 * Dashboard Builder: pick (or create) a dashboard, then pin tiles to live
 * points or boards through the real `/api/v1/widgets` store. A dashboard is an
 * **org overview** (spans every site) or **site-scoped**; tiles persist as
 * server rows under the board, so they survive reload.
 */
export function Builder() {
  // Org comes from the URL scope (`/o/$org/...`), so the page works on the
  // org-level dashboards route where no single site is selected. `site` is the
  // active site when on a site route; `sites` scopes the create-dialog picker.
  const { org, site, sites } = useScope()

  // Bind the time store to the URL (`?from/to/refresh`) and run the auto-refresh
  // loop (paused on a hidden tab) for every widget on the open dashboard.
  useTimeRangeSync()

  const { data: dashboards = [] } = useDashboards(org)
  const [pickedId, setPickedId] = useState<string | undefined>()
  const selectedId = pickedId ?? dashboards[0]?.id
  const selected = dashboards.find((d) => d.id === selectedId)

  const orgSites = sites.filter((s) => s.org === org)
  // The site the widget binder resolves its point cascade against. A site
  // dashboard binds to its own site; an org overview (no site_id) has no single
  // site, so default to the active site, else the org's first — the operator can
  // switch sites in the binder to pin tiles from any site onto the overview.
  const [bindSiteId, setBindSiteId] = useState<string | undefined>()
  const dashboardSite = selected?.site_id
    ? orgSites.find((s) => s.id === selected.site_id)
    : undefined
  const bindSite =
    dashboardSite ??
    site ??
    orgSites.find((s) => s.id === bindSiteId) ??
    orgSites[0]

  const [picked, setPicked] = useState<Bindable | null>(null)
  const [formOpen, setFormOpen] = useState(false)
  const [editing, setEditing] = useState<Dashboard | null>(null)
  const [editVarsOpen, setEditVarsOpen] = useState(false)

  // Variable resolution + URL state for the selected dashboard. The bar drives
  // the `?var-*` selection; widgets re-query off the resolved values.
  const { selection, setSelection } = useVarUrlState()
  // Assemble the page context for the open board (nav node + bare URL params +
  // tags + scope) and fold it into resolution so the same board, mounted at two
  // nav nodes, resolves two sites' data (docs/design/page-context-and-nav.md §1).
  const pageContext = usePageContext({
    org,
    siteId: selected?.site_id ?? undefined,
    dashboardId: selected?.id,
    boardSlug: selected?.slug,
  })
  const { resolved, error: varError } = useVariableResolution({
    org,
    variables: selected?.variables ?? [],
    selection,
    pageContext,
  })

  return (
    <>
      <PageHeader
        title='Dashboard Builder'
        sub='Compose dashboards for an org or a site'
      />
      <Main fluid fixed className='flex min-h-0 flex-col'>
        <div className='mb-3 flex items-center gap-2'>
          <DashboardPicker
            dashboards={dashboards}
            selectedId={selectedId}
            onSelect={setPickedId}
            onNew={() => setFormOpen(true)}
            onEdit={(d) => setEditing(d)}
          />
          {selected ? (
            <Button
              size='sm'
              variant='outline'
              onClick={() => setEditVarsOpen(true)}
            >
              <SlidersHorizontal className='size-3.5' /> Variables
            </Button>
          ) : null}
          {selected ? <TagEditor dashboardId={selected.id} /> : null}
          {selected ? (
            <HistoryButton
              kind='dashboard'
              id={selected.id}
              resourceName={selected.title}
            />
          ) : null}
          <div className='ms-auto'>
            <TimeRangePicker />
          </div>
        </div>

        {selected && resolved.some((r) => !r.variable.hidden) ? (
          <div className='mb-3'>
            <VariableBar
              resolved={resolved}
              error={varError}
              onChange={setSelection}
            />
          </div>
        ) : varError ? (
          <div className='mb-3'>
            <VariableBar
              resolved={[]}
              error={varError}
              onChange={setSelection}
            />
          </div>
        ) : null}

        <div className='grid min-h-0 w-full flex-1 gap-3 lg:grid-cols-[230px_1fr]'>
          <Card className='scroll overflow-y-auto p-2.5'>
            {selected && bindSite ? (
              <WidgetPalette onPick={setPicked} />
            ) : (
              <p className='text-[12px] text-muted-foreground'>
                {dashboards.length === 0
                  ? 'Create a dashboard to begin.'
                  : !bindSite
                    ? 'Add a site to this org to pin widgets.'
                    : 'Select a dashboard.'}
              </p>
            )}
          </Card>

          <div className='scroll min-h-0 overflow-y-auto pe-1'>
            {selected ? (
              <DashboardCanvas dashboard={selected} sites={sites} />
            ) : (
              <Card className='grid h-full place-items-center'>
                <div className='max-w-xs text-center'>
                  <p className='text-[13px] font-medium'>
                    No dashboard selected
                  </p>
                  <p className='mt-1 text-[12px] text-muted-foreground'>
                    Create an org overview or a site dashboard to start pinning
                    tiles.
                  </p>
                </div>
              </Card>
            )}
          </div>
        </div>
      </Main>

      {/* Pin tiles onto the selected dashboard. `bindSite` resolves the point
          cascade — the dashboard's own site, or (for an overview) the active /
          first site, switchable in the binder so an overview can mix sites. */}
      {selected && bindSite ? (
        <WidgetBinder
          site={bindSite}
          dashboardId={selected.id}
          entry={picked}
          onClose={() => setPicked(null)}
          // An overview (no dashboard site) lets the operator pick which site to
          // pin from; a site dashboard is fixed to its site.
          sites={selected.site_id ? undefined : orgSites}
          onSiteChange={setBindSiteId}
        />
      ) : null}

      {org ? (
        <DashboardFormDialog
          mode='create'
          open={formOpen}
          onOpenChange={setFormOpen}
          org={org}
          sites={sites}
          onCreated={setPickedId}
        />
      ) : null}

      {editing && org ? (
        <DashboardFormDialog
          mode='edit'
          dashboard={editing}
          open={editing !== null}
          onOpenChange={(o) => !o && setEditing(null)}
          org={org}
          sites={sites}
        />
      ) : null}

      {selected ? (
        <VariableEditorDialog
          open={editVarsOpen}
          onOpenChange={setEditVarsOpen}
          org={org}
          dashboard={selected}
        />
      ) : null}
    </>
  )
}

/**
 * Renders one dashboard's tiles. A site board resolves keyexprs against that
 * site's points; an overview resolves against the union of all the org's sites.
 */
function DashboardCanvas({
  dashboard,
  sites,
}: {
  dashboard: Dashboard
  sites: Site[]
}) {
  const { data: widgets = [] } = useWidgets({ dashboardId: dashboard.id })
  if (dashboard.site_id) {
    return (
      <SiteCanvas siteId={dashboard.site_id} widgets={widgets} sites={sites} />
    )
  }
  return <OverviewCanvas org={dashboard.org} widgets={widgets} sites={sites} />
}

function SiteCanvas({
  siteId,
  widgets,
  sites,
}: {
  siteId: string
  widgets: Widget[]
  sites: Site[]
}) {
  const site = sites.find((s) => s.id === siteId)
  const { data: equips = [] } = useEquips(siteId)
  const { data: points = [] } = usePoints({ siteId })
  const index = useMemo(
    () =>
      site ? keyexprIndex(site, equips, points) : new Map<string, never>(),
    [site, equips, points]
  )
  return <WidgetCanvas widgets={widgets} index={index} />
}

function OverviewCanvas({
  org,
  widgets,
  sites,
}: {
  org: string
  widgets: Widget[]
  sites: Site[]
}) {
  // An overview spans the org's sites; resolve targets against all of them.
  const orgSites = useMemo(
    () => sites.filter((s) => s.org === org),
    [sites, org]
  )
  const { data: equips = [] } = useEquips()
  const { data: points = [] } = usePoints({})
  const index = useMemo(
    () => keyexprIndexMulti(orgSites, equips, points),
    [orgSites, equips, points]
  )
  return <WidgetCanvas widgets={widgets} index={index} />
}
