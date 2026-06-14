/**
 * React Query hooks over the endpoint functions. Reads poll on an interval so
 * live `cur` values and new sparks surface without a manual refresh; mutations
 * invalidate the affected keys.
 */
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useEffect, useRef, useState } from 'react'

import * as api from './endpoints'
import { qk } from './keys'
import { streamBoardOutputs } from './stream'
import type {
  CreateEquip,
  CreatePoint,
  CreateRule,
  CreateSite,
  CreateDashboard,
  CreateWidget,
  DryRunRequest,
  PatchDashboard,
  CurRequest,
  PatchBoard,
  PatchEquip,
  PatchPoint,
  PatchSite,
  PatchWidget,
  PortOutput,
  PreferencesPatch,
  ProvisionOrg,
  QueryVariable,
  RunStatus,
  TimeRangeBody,
  UpdateRule,
  Uuid,
  Widget,
  WriteRequest,
  CreateUser,
  PatchUser,
  CreateTeam,
  PatchTeam,
  CreateGrant,
  CreateDashboardGrant,
  AuditQuery,
} from './types'
import { invalidationKeys } from '@/features/audit/invalidate'

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

/**
 * A point's history. With a `time` arg (the resolved dashboard range) the read
 * is bounded server-side to `start`/`end` and keyed on the snapped range so the
 * tile fetches only the in-range span and re-fetches on a range/refresh change
 * (docs/design/time-range-and-refresh.md §4); without it, the prior live-poll
 * behaviour is preserved (whole window, 5 s poll).
 */
export function usePointHistory(
  id: Uuid | undefined,
  time?: {
    start: string
    end: string
    tickKey: string
    refreshSecs: number
  }
) {
  // `time.tickKey` is the snapped range + refresh tick, the canonical cache
  // discriminator; the `start`/`end` it derives need not also list in the key.
  // eslint-disable-next-line @tanstack/query/exhaustive-deps
  return useQuery({
    queryKey: qk.pointHistory(id ?? 'none', time?.tickKey),
    queryFn: ({ signal }) =>
      api.points.history(
        id as Uuid,
        time ? { start: time.start, end: time.end } : undefined,
        signal
      ),
    enabled: Boolean(id),
    refetchInterval: time
      ? time.refreshSecs > 0
        ? time.refreshSecs * 1000
        : false
      : LIVE_INTERVAL,
  })
}

export function useSparks(siteId?: Uuid) {
  return useQuery({
    queryKey: qk.sparks(siteId),
    queryFn: ({ signal }) => api.sparks.list(siteId, signal),
    refetchInterval: LIVE_INTERVAL,
  })
}

/**
 * Agent runs, optionally narrowed to one lifecycle status (server-side
 * `?status=`). Polls while any returned run is suspended so the approval queue
 * stays live without a manual refresh.
 */
export function useRuns(status?: RunStatus) {
  return useQuery({
    queryKey: qk.runsByStatus(status),
    queryFn: ({ signal }) => api.runs.list(status, signal),
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

/**
 * The embedded agent's config (enabled, provider/model, priority gate). Global
 * and env-set at boot, so it rarely changes — a long stale time avoids
 * refetching on every Runs-page mount.
 */
export function useAgentStatus() {
  return useQuery({
    queryKey: qk.agentStatus,
    queryFn: ({ signal }) => api.agent.status(signal),
    staleTime: 60_000,
  })
}

/**
 * Flows in a scope. `org` required; `siteId` optional — with it, the site's
 * flows + the org-level ones; without, every flow the org owns. Disabled until
 * an org resolves (so an org-level page with no site still lists).
 */
export function useBoards(org: string | undefined, siteId?: Uuid) {
  return useQuery({
    queryKey: org ? [...qk.boards, org, siteId ?? 'org'] : qk.boards,
    queryFn: ({ signal }) =>
      api.boards.list({ org: org as string, siteId }, signal),
    enabled: Boolean(org),
  })
}

/** Run a stored flow on demand within its scope; resolves the outport packets. */
export function useRunStoredBoard() {
  return useMutation({
    mutationFn: ({
      slug,
      org,
      siteId,
    }: {
      slug: string
      org: string
      siteId?: Uuid
    }) => api.boards.runStored(slug, { org, siteId }),
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
export function useBoardOutputs(
  slug: string | undefined,
  scope: { org: string | undefined; siteId?: Uuid },
  live: boolean
) {
  return useQuery({
    queryKey: [
      ...qk.boardOutputs(slug ?? 'none'),
      scope.org,
      scope.siteId ?? 'org',
    ],
    queryFn: ({ signal }) =>
      api.boards.outputs(
        slug as string,
        { org: scope.org as string, siteId: scope.siteId },
        signal
      ),
    enabled: Boolean(slug && scope.org),
    refetchInterval: live ? LIVE_INTERVAL : false,
  })
}

/** Backoff before reconnecting a dropped board outputs stream. */
const STREAM_RECONNECT_MS = 2_000

/**
 * Subscribe to a board's live output values over SSE (real-time, replacing the
 * 5s poll of {@link useBoardOutputs}). Seeds from the snapshot the server sends
 * on connect, then folds each subsequent snapshot into a retained
 * last-known-good map keyed by `(node, port)` — so a momentary empty run does
 * not blank the canvas, and a dropped connection keeps the last values until it
 * reconnects. Inactive (and emits nothing) when `live` is false.
 */
export function useBoardOutputsStream(
  slug: string | undefined,
  scope: { org: string | undefined; siteId?: Uuid },
  live: boolean
): { data: PortOutput[] } {
  const org = scope.org
  const siteId = scope.siteId
  // The active subscription identity; empty when not streaming.
  const activeKey = live && slug && org ? `${slug}|${org}|${siteId ?? ''}` : ''

  // Snapshot tagged with the subscription it belongs to, plus the retained
  // last-known-good map. Tagging lets a board/scope switch derive an empty view
  // (below) without a synchronous setState in the effect.
  const [snapshot, setSnapshot] = useState<{ key: string; items: PortOutput[] }>({
    key: '',
    items: [],
  })
  const retained = useRef<{ key: string; map: Map<string, PortOutput> }>({
    key: '',
    map: new Map(),
  })

  useEffect(() => {
    if (!live || !slug || !org) return
    const key = `${slug}|${org}|${siteId ?? ''}`
    // A fresh subscription owns its own retained map (ref write — no render).
    retained.current = { key, map: new Map() }

    const controller = new AbortController()
    let stopped = false

    const consume = (incoming: PortOutput[]) => {
      if (retained.current.key !== key) return
      for (const out of incoming) {
        retained.current.map.set(`${out.node} ${out.port}`, out)
      }
      setSnapshot({ key, items: Array.from(retained.current.map.values()) })
    }

    const run = async () => {
      while (!stopped) {
        try {
          await streamBoardOutputs(slug, { org, siteId }, consume, controller.signal)
        } catch {
          // Network drop or non-OK response — fall through to backoff/reconnect.
        }
        if (stopped) break
        await new Promise((resolve) => setTimeout(resolve, STREAM_RECONNECT_MS))
      }
    }
    void run()

    return () => {
      stopped = true
      controller.abort()
    }
  }, [slug, org, siteId, live])

  // Only surface the snapshot that belongs to the current subscription; a switch
  // shows nothing until the new stream's first frame, never the prior board's.
  const data = snapshot.key === activeKey ? snapshot.items : []
  return { data }
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

/**
 * Dropdown choices for a config field's `option_source`, scoped to the board's
 * `{org}/{site}`. `enabled` lets a caller hold the fetch until the dropdown is
 * opened (and until any required `datasource` narrowing is known). The client
 * stays agnostic to what `source` means — the server resolves it.
 */
export function useBoardOptions(
  source: string | undefined,
  scope: { org?: string; site?: string; datasource?: string },
  enabled = true,
) {
  return useQuery({
    queryKey: qk.boardOptions(source ?? '', scope),
    queryFn: ({ signal }) => api.boards.options(source!, scope, signal),
    enabled: Boolean(source) && enabled && Boolean(scope.org),
    staleTime: 30_000,
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

/** Patch a flow's latest-version metadata (`display_name`/`enabled`) in scope. */
export function usePatchBoard() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({
      slug,
      org,
      siteId,
      body,
    }: {
      slug: string
      org: string
      siteId?: Uuid
      body: PatchBoard
    }) => api.boards.patch(slug, { org, siteId }, body),
    onSuccess: (board) => {
      qc.invalidateQueries({ queryKey: qk.boards })
      qc.invalidateQueries({ queryKey: qk.board(board.slug) })
    },
  })
}

/** Delete every version of a flow slug within its scope; refresh the list. */
export function useDeleteBoard() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({
      slug,
      org,
      siteId,
    }: {
      slug: string
      org: string
      siteId?: Uuid
    }) => api.boards.remove(slug, { org, siteId }),
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

/**
 * A `datasource` widget's SQL result, resolved against the dashboard's current
 * variable selection. The query keys on `varRevision` (a hash of the values of
 * the variables this SQL references) so a selection change re-fetches exactly
 * the dependent widgets and leaves unreferenced ones untouched
 * (variables-and-templating.md §6). Disabled until SQL is present; polls live so
 * the tile stays fresh like the other reads.
 */
export function useWidgetData(args: {
  widgetId: Uuid
  sql: string | undefined
  variables: QueryVariable[]
  varRevision: string
  /**
   * The dashboard time range + derived bucket the SQL's time macros bind against
   * (docs/design/time-range-and-refresh.md §4). `tickKey` is the snapped range +
   * refresh tick folded into the cache key so a range/refresh change re-fetches
   * cleanly; omit to fall back to the prior whole-window 5 s live poll.
   */
  time?: {
    timeRange: TimeRangeBody
    intervalSecs: number
    tickKey: string
    refreshSecs: number
  }
}) {
  const { widgetId, sql, variables, varRevision, time } = args
  // `varRevision` and the time `tickKey` are hashes of the resolved values the
  // `sql`/`variables`/range carry, so they are the canonical cache
  // discriminators; listing `sql`/`variables`/`time` too would be redundant (and
  // they are objects that change identity each render).
  // eslint-disable-next-line @tanstack/query/exhaustive-deps
  return useQuery({
    queryKey: qk.widgetData(widgetId, varRevision, time?.tickKey),
    queryFn: () =>
      api.query.run(sql as string, {
        variables,
        ...(time
          ? { timeRange: time.timeRange, intervalSecs: time.intervalSecs }
          : {}),
      }),
    enabled: Boolean(sql),
    refetchInterval: time
      ? time.refreshSecs > 0
        ? time.refreshSecs * 1000
        : false
      : LIVE_INTERVAL,
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

/**
 * Update a tile's presentation `settings` (grid layout + chart config). Used by
 * the canvas on drag/resize and by the binder on a chart-type change. The
 * mutation is optimistic so the tile snaps to its new cell without a round-trip
 * flicker; on settle the widget lists refresh to the server truth.
 */
export function usePatchWidget() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({ id, body }: { id: Uuid; body: PatchWidget }) =>
      api.widgets.patch(id, body),
    onMutate: async ({ id, body }) => {
      await qc.cancelQueries({ queryKey: ['widgets'] })
      const prev = qc.getQueriesData<Widget[]>({ queryKey: ['widgets'] })
      for (const [key, list] of prev) {
        if (!list) continue
        qc.setQueryData<Widget[]>(
          key,
          list.map((w) =>
            w.id === id ? { ...w, settings: body.settings ?? undefined } : w
          )
        )
      }
      return { prev }
    },
    onError: (_e, _vars, ctx) => {
      // Roll back to the pre-mutation snapshots on failure.
      ctx?.prev.forEach(([key, list]) => qc.setQueryData(key, list))
    },
    onSettled: () => qc.invalidateQueries({ queryKey: ['widgets'] }),
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

// --- Rules engine (Rules Studio) ----------------------------------------------

/**
 * Stored rules in a scope. `org` required; `siteId` optional — with it, the
 * site's rules + the org-level ones; without, every rule the org owns.
 */
export function useRules(org: string | undefined, siteId?: Uuid) {
  return useQuery({
    queryKey: [...qk.rules(org), siteId ?? 'org'],
    queryFn: ({ signal }) => api.rules.list(org as string, siteId, signal),
    enabled: Boolean(org),
  })
}

/** One stored rule by name at an exact scope. */
export function useRule(
  org: string | undefined,
  name: string | undefined,
  siteId?: Uuid
) {
  return useQuery({
    queryKey:
      org && name
        ? [...qk.rule(org, name), siteId ?? 'org']
        : ['rules', 'none'],
    queryFn: ({ signal }) =>
      api.rules.get(org as string, name as string, siteId, signal),
    enabled: Boolean(org && name),
  })
}

/**
 * Rules that compose this one — the change-impact list surfaced before an edit
 * or delete. Only fetched when a rule is selected.
 */
export function useReferencingRules(
  org: string | undefined,
  name: string | undefined,
  siteId?: Uuid
) {
  return useQuery({
    queryKey:
      org && name
        ? [...qk.ruleReferencing(org, name), siteId ?? 'org']
        : ['rules', 'none', 'ref'],
    queryFn: ({ signal }) =>
      api.rules.referencing(org as string, name as string, siteId, signal),
    enabled: Boolean(org && name),
  })
}

/** Create a rule under an org (optionally a site, via body.site_id); refresh. */
export function useCreateRule(org: string | undefined) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (body: CreateRule) => api.rules.create(org as string, body),
    onSuccess: () => qc.invalidateQueries({ queryKey: qk.rules(org) }),
  })
}

/** Update a rule's script/params at its scope; refresh the list and the rule. */
export function useUpdateRule(org: string | undefined) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({
      name,
      siteId,
      body,
    }: {
      name: string
      siteId?: Uuid
      body: UpdateRule
    }) => api.rules.update(org as string, name, siteId, body),
    onSuccess: (rule) => {
      qc.invalidateQueries({ queryKey: qk.rules(org) })
      if (org) qc.invalidateQueries({ queryKey: qk.rule(org, rule.name) })
    },
  })
}

/** Delete a rule by name at its scope; refresh the org's rule list. */
export function useDeleteRule(org: string | undefined) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({ name, siteId }: { name: string; siteId?: Uuid }) =>
      api.rules.remove(org as string, name, siteId),
    onSuccess: () => qc.invalidateQueries({ queryKey: qk.rules(org) }),
  })
}

/**
 * Dry-run a rule against a point's history without emitting a spark. The
 * debugger's tight edit→run loop drives this; it is not cached (each run is a
 * fresh evaluation of the current script).
 */
export function useDryRunRule(org: string | undefined) {
  return useMutation({
    mutationFn: (body: DryRunRequest) => api.rules.dryRun(org as string, body),
  })
}

// --- Authorization ------------------------------------------------------------

/**
 * The caller's resolved identity + capabilities. Read once at boot to render
 * permission-aware chrome (admin nav, disabled write controls). Cached long —
 * identity does not change within a session. On the open dev server this
 * resolves to a synthetic global operator (`auth_enabled: false`).
 */
export function useWhoami() {
  return useQuery({
    queryKey: qk.whoami,
    queryFn: ({ signal }) => api.auth.whoami(signal),
    staleTime: Infinity,
  })
}

// --- RBAC admin surfaces: users, teams, grants (authz-rbac.md increment E) -----

export function useUsers(org: string | undefined) {
  return useQuery({
    queryKey: qk.users(org),
    queryFn: ({ signal }) => api.users.list(org as string, signal),
    enabled: Boolean(org),
  })
}

export function useCreateUser(org: string) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (body: CreateUser) => api.users.create(org, body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['users'] }),
  })
}

export function usePatchUser(org: string) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({ id, body }: { id: Uuid; body: PatchUser }) =>
      api.users.patch(org, id, body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['users'] }),
  })
}

export function useDeleteUser(org: string) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (id: Uuid) => api.users.remove(org, id),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['users'] })
      qc.invalidateQueries({ queryKey: ['teams'] })
    },
  })
}

export function useTeams(org: string | undefined) {
  return useQuery({
    queryKey: qk.teams(org),
    queryFn: ({ signal }) => api.teams.list(org as string, signal),
    enabled: Boolean(org),
  })
}

export function useCreateTeam(org: string) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (body: CreateTeam) => api.teams.create(org, body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['teams'] }),
  })
}

export function usePatchTeam(org: string) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({ id, body }: { id: Uuid; body: PatchTeam }) =>
      api.teams.patch(org, id, body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['teams'] }),
  })
}

export function useDeleteTeam(org: string) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (id: Uuid) => api.teams.remove(org, id),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['teams'] }),
  })
}

/** The members of a team. */
export function useTeamMembers(org: string, teamId: Uuid | undefined) {
  return useQuery({
    queryKey: qk.teamMembers(org, teamId as Uuid),
    queryFn: ({ signal }) => api.teams.members(org, teamId as Uuid, signal),
    enabled: Boolean(teamId),
  })
}

export function useAddTeamMember(org: string, teamId: Uuid) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (userId: Uuid) => api.teams.addMember(org, teamId, userId),
    onSuccess: () =>
      qc.invalidateQueries({ queryKey: qk.teamMembers(org, teamId) }),
  })
}

export function useRemoveTeamMember(org: string, teamId: Uuid) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (userId: Uuid) => api.teams.removeMember(org, teamId, userId),
    onSuccess: () =>
      qc.invalidateQueries({ queryKey: qk.teamMembers(org, teamId) }),
  })
}

/** Grants in an org, optionally filtered to one resource ref. */
export function useGrants(org: string | undefined, resourceRef?: string) {
  return useQuery({
    queryKey: qk.grants(org, resourceRef),
    queryFn: ({ signal }) =>
      api.grants.list(org as string, resourceRef, signal),
    enabled: Boolean(org),
  })
}

export function useCreateGrant(org: string) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (body: CreateGrant) => api.grants.create(org, body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['grants'] }),
  })
}

export function useDeleteGrant(org: string) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (id: Uuid) => api.grants.remove(org, id),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['grants'] }),
  })
}

/** Grants pinned to a single dashboard (the Access page's per-resource view). */
export function useDashboardGrants(id: Uuid | undefined) {
  return useQuery({
    queryKey: qk.dashboardGrants(id as Uuid),
    queryFn: ({ signal }) => api.grants.forDashboard(id as Uuid, signal),
    enabled: Boolean(id),
  })
}

export function useGrantDashboard(dashboardId: Uuid) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (body: CreateDashboardGrant) =>
      api.grants.grantDashboard(dashboardId, body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['grants'] }),
  })
}

// --- Audit & undo/redo (docs/design/audit-and-undo.md) ------------------------

/**
 * The org's audit log, narrowed by the active filters. Admin-gated server-side
 * (`require_admin(org)`); the screen also hides itself for a non-`can_admin`
 * caller. Disabled until an org resolves. The query key folds the filter shape so
 * a filter change refetches without colliding with another filter's page.
 */
export function useAudit(org: string | undefined, filter: Omit<AuditQuery, 'org'>) {
  const filterKey = JSON.stringify(filter)
  // `filterKey` is the canonical serialization of `filter`, so it is the cache
  // discriminator; listing the `filter` object too would be redundant (a fresh
  // object identity each render).
  // eslint-disable-next-line @tanstack/query/exhaustive-deps
  return useQuery({
    queryKey: qk.audit(org ?? 'none', filterKey),
    queryFn: ({ signal }) =>
      api.audit.list({ org: org as string, ...filter }, signal),
    enabled: Boolean(org),
  })
}

/**
 * One resource's change timeline — the per-resource History tab (timeline +
 * before→after diff). Only fetched when a resource is selected.
 */
export function useResourceHistory(
  kind: string | undefined,
  id: Uuid | undefined
) {
  return useQuery({
    queryKey: qk.auditTimeline(kind ?? 'none', id ?? ('none' as Uuid)),
    queryFn: ({ signal }) => api.audit.timeline(kind as string, id as Uuid, signal),
    enabled: Boolean(kind && id),
  })
}

/**
 * Undo the caller's most-recent change group in `org`. On success the returned
 * touched-resource ids drive precise invalidation so the canvas refreshes exactly
 * what moved, and the audit log itself refreshes.
 */
export function useUndo(org: string | undefined) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: () => api.audit.undo(org as string),
    onSuccess: (result) => {
      for (const queryKey of invalidationKeys(result)) {
        qc.invalidateQueries({ queryKey })
      }
      qc.invalidateQueries({ queryKey: ['audit'] })
    },
  })
}

/** Redo the caller's most-recently-undone change group; same invalidation path. */
export function useRedo(org: string | undefined) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: () => api.audit.redo(org as string),
    onSuccess: (result) => {
      for (const queryKey of invalidationKeys(result)) {
        qc.invalidateQueries({ queryKey })
      }
      qc.invalidateQueries({ queryKey: ['audit'] })
    },
  })
}

// --- Units & datetime preferences (WS-11) ------------------------------------

/** The caller's fully-resolved preferences (`GET /api/v1/me/preferences`). */
export function useMyPreferences() {
  return useQuery({
    queryKey: qk.myPreferences,
    queryFn: ({ signal }) => api.preferences.getMe(signal),
    // Prefs change rarely; no polling. Cache generously.
    staleTime: 5 * 60_000,
  })
}

/** Patch the caller's preferences; on success the resolved view is refreshed. */
export function useUpdateMyPreferences() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (body: PreferencesPatch) => api.preferences.patchMe(body),
    onSuccess: (resolved) => {
      // Seed the cache with the server's re-resolved view so the provider
      // updates without a second round-trip.
      qc.setQueryData(qk.myPreferences, resolved)
    },
  })
}

/** The closed unit registry (`GET /api/v1/units`) — quantity pickers read it. */
export function useUnits() {
  return useQuery({
    queryKey: qk.units,
    queryFn: ({ signal }) => api.units.list(signal),
    staleTime: Infinity, // closed registry; never changes for a build
  })
}
