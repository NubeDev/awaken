/**
 * React Query hooks over the endpoint functions. Reads poll on an interval so
 * live `cur` values and new sparks surface without a manual refresh; mutations
 * invalidate the affected keys.
 */
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import * as api from './endpoints'
import { qk } from './keys'
import type {
  CreateEquip,
  CreatePoint,
  CreateSite,
  CreateDashboard,
  CreateWidget,
  PatchDashboard,
  CurRequest,
  PatchBoard,
  PatchEquip,
  PatchPoint,
  PatchSite,
  ProvisionOrg,
  Uuid,
  WriteRequest,
} from './types'

const LIVE_INTERVAL = 5_000

export function useSites(org?: string) {
  return useQuery({
    queryKey: org ? [...qk.sites, org] : qk.sites,
    queryFn: ({ signal }) => api.sites.list(org, signal),
  })
}

export function useOrgs(org?: string) {
  return useQuery({
    queryKey: org ? [...qk.orgs, org] : qk.orgs,
    queryFn: ({ signal }) => api.orgs.list(org, signal),
  })
}

/** Provision a tenant (org + first site); refresh the org and site lists. */
export function useProvisionOrg() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (body: ProvisionOrg) => api.orgs.provision(body),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: qk.orgs })
      qc.invalidateQueries({ queryKey: qk.sites })
    },
  })
}

export function useCreateSite() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (body: CreateSite) => api.sites.create(body),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: qk.sites })
      qc.invalidateQueries({ queryKey: qk.orgs })
    },
  })
}

export function usePatchSite() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({ id, body }: { id: Uuid; body: PatchSite }) =>
      api.sites.patch(id, body),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: qk.sites })
      qc.invalidateQueries({ queryKey: qk.orgs })
    },
  })
}

/** Delete a site (cascades server-side); refresh the site and org lists. */
export function useDeleteSite() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (id: Uuid) => api.sites.remove(id),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: qk.sites })
      qc.invalidateQueries({ queryKey: qk.orgs })
    },
  })
}

export function useEquips(siteId?: Uuid) {
  return useQuery({
    queryKey: qk.equips(siteId),
    queryFn: ({ signal }) => api.equips.list(siteId, signal),
  })
}

export function useCreateEquip() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (body: CreateEquip) => api.equips.create(body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['equips'] }),
  })
}

export function usePatchEquip() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({ id, body }: { id: Uuid; body: PatchEquip }) =>
      api.equips.patch(id, body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['equips'] }),
  })
}

/** Delete an equip (cascades to its points); refresh equips and points. */
export function useDeleteEquip() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (id: Uuid) => api.equips.remove(id),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['equips'] })
      qc.invalidateQueries({ queryKey: ['points'] })
    },
  })
}

export function usePoints(params: {
  equipId?: Uuid
  siteId?: Uuid
  tags?: string
}) {
  return useQuery({
    queryKey: qk.points(params),
    queryFn: ({ signal }) => api.points.list(params, signal),
    refetchInterval: LIVE_INTERVAL,
  })
}

export function useCreatePoint() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (body: CreatePoint) => api.points.create(body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['points'] }),
  })
}

export function usePatchPoint() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({ id, body }: { id: Uuid; body: PatchPoint }) =>
      api.points.patch(id, body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['points'] }),
  })
}

export function useDeletePoint() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (id: Uuid) => api.points.remove(id),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['points'] }),
  })
}

export function usePointHistory(id: Uuid | undefined) {
  return useQuery({
    queryKey: qk.pointHistory(id ?? 'none'),
    queryFn: ({ signal }) => api.points.history(id as Uuid, signal),
    enabled: Boolean(id),
    refetchInterval: LIVE_INTERVAL,
  })
}

export function useSparks(siteId?: Uuid) {
  return useQuery({
    queryKey: qk.sparks(siteId),
    queryFn: ({ signal }) => api.sparks.list(siteId, signal),
    refetchInterval: LIVE_INTERVAL,
  })
}

/** Poll the run list while any run is suspended so the approval queue stays live. */
export function useRuns() {
  return useQuery({
    queryKey: qk.runs,
    queryFn: ({ signal }) => api.runs.list(signal),
    refetchInterval: (query) =>
      (query.state.data ?? []).some((r) => r.status === 'suspended')
        ? LIVE_INTERVAL
        : false,
  })
}

/** One run; polls while suspended so an approval landing elsewhere reflects here. */
export function useRun(id: string | undefined) {
  return useQuery({
    queryKey: qk.run(id ?? 'none'),
    queryFn: ({ signal }) => api.runs.get(id as string, signal),
    enabled: Boolean(id),
    refetchInterval: (query) =>
      query.state.data?.status === 'suspended' ? LIVE_INTERVAL : false,
  })
}

/**
 * Approve a suspended run: the agent's held write is re-applied through the
 * priority array. Invalidate runs and points so the resumed status and the
 * agent's write at its priority slot both surface.
 */
export function useResumeRun() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (id: string) => api.runs.resume(id),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['runs'] })
      qc.invalidateQueries({ queryKey: ['points'] })
    },
  })
}

/** Reject a suspended run: the held write is discarded; refresh the run state. */
export function useCancelRun() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (id: string) => api.runs.cancel(id),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['runs'] }),
  })
}

export function useAckSpark(siteId?: Uuid) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (id: Uuid) => api.sparks.ack(id),
    onSuccess: () => qc.invalidateQueries({ queryKey: qk.sparks(siteId) }),
  })
}

/** Delete a finding; refresh the site's spark list. */
export function useDeleteSpark(siteId?: Uuid) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (id: Uuid) => api.sparks.remove(id),
    onSuccess: () => qc.invalidateQueries({ queryKey: qk.sparks(siteId) }),
  })
}

export function useWritePoint() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({ id, body }: { id: Uuid; body: WriteRequest }) =>
      api.points.write(id, body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['points'] }),
  })
}

export function useRelinquishPoint() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({ id, priority }: { id: Uuid; priority: number }) =>
      api.points.relinquish(id, priority),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['points'] }),
  })
}

export function useIngestPoint() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({ id, body }: { id: Uuid; body: CurRequest }) =>
      api.points.ingest(id, body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['points'] }),
  })
}

export function useAgentChat() {
  return useMutation({ mutationFn: api.agent.chat })
}

export function useBoards() {
  return useQuery({
    queryKey: qk.boards,
    queryFn: ({ signal }) => api.boards.list(signal),
  })
}

/** Run a stored board on demand; resolves the run's outport packets. */
export function useRunStoredBoard() {
  return useMutation({
    mutationFn: (slug: string) => api.boards.runStored(slug),
  })
}

/**
 * Run an inline graph once — evaluates exactly what's on the editor canvas,
 * including unsaved edits, so Test Run reflects the current node set.
 */
export function useRunInlineBoard() {
  return useMutation({ mutationFn: api.boards.runInline })
}

/**
 * Latest per-node values a board has produced (scheduler cache). Polls while
 * `live` so an enabled board's autonomous runs surface on the canvas without a
 * manual Test Run; pass `live=false` (e.g. a manual board) to fetch once.
 */
export function useBoardOutputs(slug: string | undefined, live: boolean) {
  return useQuery({
    queryKey: qk.boardOutputs(slug ?? 'none'),
    queryFn: ({ signal }) => api.boards.outputs(slug as string, signal),
    enabled: Boolean(slug),
    refetchInterval: live ? LIVE_INTERVAL : false,
  })
}

/**
 * The board component catalogue (ports + config schema). Static for a server
 * build, so it never refetches; the flow editor's palette and config form read
 * from it.
 */
export function useBoardComponents() {
  return useQuery({
    queryKey: qk.boardComponents,
    queryFn: ({ signal }) => api.boards.components(signal),
    staleTime: Infinity,
  })
}

/** Save a board edit as a new version of the slug; refreshes the board list. */
export function useSaveBoard() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: api.boards.save,
    onSuccess: (board) => {
      qc.invalidateQueries({ queryKey: qk.boards })
      qc.invalidateQueries({ queryKey: qk.board(board.slug) })
    },
  })
}

/** Patch a board's latest-version metadata (`display_name`/`enabled`). */
export function usePatchBoard() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({ slug, body }: { slug: string; body: PatchBoard }) =>
      api.boards.patch(slug, body),
    onSuccess: (board) => {
      qc.invalidateQueries({ queryKey: qk.boards })
      qc.invalidateQueries({ queryKey: qk.board(board.slug) })
    },
  })
}

/** Delete every version of a board slug; refresh the board list. */
export function useDeleteBoard() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (slug: string) => api.boards.remove(slug),
    onSuccess: () => qc.invalidateQueries({ queryKey: qk.boards }),
  })
}

/**
 * Pinned widgets, filtered by site and/or dashboard; polls so live tiles stay
 * fresh. The builder passes `dashboardId` to scope tiles to the open board.
 */
export function useWidgets(params: { siteId?: Uuid; dashboardId?: Uuid } = {}) {
  return useQuery({
    queryKey: qk.widgets(params),
    queryFn: ({ signal }) => api.widgets.list(params, signal),
    refetchInterval: LIVE_INTERVAL,
  })
}

/** Pin a widget; invalidate widget lists so it appears on the canvas. */
export function useCreateWidget() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (body: CreateWidget) => api.widgets.create(body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['widgets'] }),
  })
}

/** Remove a pinned widget from the canvas; refresh widget lists. */
export function useDeleteWidget() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (id: Uuid) => api.widgets.remove(id),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['widgets'] }),
  })
}

// --- Dashboards ----------------------------------------------------------------

/** Dashboards under an org, optionally filtered to one site. */
export function useDashboards(org: string | undefined, siteId?: Uuid) {
  return useQuery({
    queryKey: qk.dashboards(org, siteId),
    queryFn: ({ signal }) =>
      api.dashboards.list({ org: org as string, siteId }, signal),
    enabled: Boolean(org),
  })
}

export function useCreateDashboard() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (body: CreateDashboard) => api.dashboards.create(body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['dashboards'] }),
  })
}

export function usePatchDashboard() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({ id, body }: { id: Uuid; body: PatchDashboard }) =>
      api.dashboards.patch(id, body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['dashboards'] }),
  })
}

/** Delete a dashboard (cascades to its tiles); refresh dashboards + widgets. */
export function useDeleteDashboard() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (id: Uuid) => api.dashboards.remove(id),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['dashboards'] })
      qc.invalidateQueries({ queryKey: ['widgets'] })
    },
  })
}
