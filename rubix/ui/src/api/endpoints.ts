/**
 * Typed endpoint functions, one per `/api/v1/*` route the UI consumes. Each is
 * a pure call into `request()`; React Query hooks wrap these for caching. The
 * only data source is the live rubix-server API — there is no fixture path.
 */
import { request } from './client'
import type {
  Whoami,
  User,
  CreateUser,
  PatchUser,
  Team,
  CreateTeam,
  PatchTeam,
  Grant,
  CreateGrant,
  CreateDashboardGrant,
  BoardGraph,
  BoardView,
  ComponentView,
  CreateBoard,
  CreateEquip,
  CreatePoint,
  CreateRule,
  CreateSite,
  AgentStatus,
  ChatRequest,
  ChatResponse,
  CurRequest,
  DryRunRequest,
  DryRunResponse,
  Equip,
  HisSample,
  OrgSummary,
  PatchBoard,
  PatchEquip,
  PatchPoint,
  PatchSite,
  Point,
  PointEnvelope,
  PortOutput,
  ProvisionOrg,
  QueryResult,
  CreateDashboard,
  CreateWidget,
  PatchWidget,
  Dashboard,
  PatchDashboard,
  ResumeResponse,
  RunStatus,
  RuleView,
  RunBoardResponse,
  RunRecord,
  Site,
  Spark,
  UpdateRule,
  Uuid,
  Widget,
  WriteRequest,
} from './types'

export const sites = {
  list: (org?: string, signal?: AbortSignal) =>
    request<Site[]>('/api/v1/sites', { query: { org }, signal }),
  get: (id: Uuid, signal?: AbortSignal) =>
    request<Site>(`/api/v1/sites/${id}`, { signal }),
  create: (body: CreateSite) =>
    request<Site>('/api/v1/sites', { method: 'POST', body }),
  patch: (id: Uuid, body: PatchSite) =>
    request<Site>(`/api/v1/sites/${id}`, { method: 'PATCH', body }),
  remove: (id: Uuid) =>
    request<void>(`/api/v1/sites/${id}`, { method: 'DELETE' }),
}

export const orgs = {
  list: (org?: string, signal?: AbortSignal) =>
    request<OrgSummary[]>('/api/v1/orgs', { query: { org }, signal }),
  provision: (body: ProvisionOrg) =>
    request<OrgSummary>('/api/v1/orgs', { method: 'POST', body }),
}

export const equips = {
  list: (siteId?: Uuid, signal?: AbortSignal) =>
    request<Equip[]>('/api/v1/equips', { query: { site_id: siteId }, signal }),
  get: (id: Uuid, signal?: AbortSignal) =>
    request<Equip>(`/api/v1/equips/${id}`, { signal }),
  create: (body: CreateEquip) =>
    request<Equip>('/api/v1/equips', { method: 'POST', body }),
  patch: (id: Uuid, body: PatchEquip) =>
    request<Equip>(`/api/v1/equips/${id}`, { method: 'PATCH', body }),
  remove: (id: Uuid) =>
    request<void>(`/api/v1/equips/${id}`, { method: 'DELETE' }),
}

export const points = {
  list: (
    params: { equipId?: Uuid; siteId?: Uuid; tags?: string },
    signal?: AbortSignal
  ) =>
    request<Point[]>('/api/v1/points', {
      query: {
        equip_id: params.equipId,
        site_id: params.siteId,
        tags: params.tags,
      },
      signal,
    }),
  get: (id: Uuid, signal?: AbortSignal) =>
    request<PointEnvelope>(`/api/v1/points/${id}`, { signal }),
  create: (body: CreatePoint) =>
    request<PointEnvelope>('/api/v1/points', { method: 'POST', body }),
  patch: (id: Uuid, body: PatchPoint) =>
    request<PointEnvelope>(`/api/v1/points/${id}`, { method: 'PATCH', body }),
  remove: (id: Uuid) =>
    request<void>(`/api/v1/points/${id}`, { method: 'DELETE' }),
  history: (id: Uuid, signal?: AbortSignal) =>
    request<HisSample[]>(`/api/v1/points/${id}/his`, { signal }),
  write: (id: Uuid, body: WriteRequest) =>
    request<PointEnvelope>(`/api/v1/points/${id}/write`, {
      method: 'POST',
      body,
    }),
  relinquish: (id: Uuid, priority: number) =>
    request<PointEnvelope>(`/api/v1/points/${id}/write/${priority}`, {
      method: 'DELETE',
    }),
  ingest: (id: Uuid, body: CurRequest) =>
    request<PointEnvelope>(`/api/v1/points/${id}/cur`, {
      method: 'POST',
      body,
    }),
}

export const sparks = {
  list: (siteId?: Uuid, signal?: AbortSignal) =>
    request<Spark[]>('/api/v1/sparks', { query: { site_id: siteId }, signal }),
  get: (id: Uuid, signal?: AbortSignal) =>
    request<Spark>(`/api/v1/sparks/${id}`, { signal }),
  ack: (id: Uuid) =>
    request<Spark>(`/api/v1/sparks/${id}/ack`, { method: 'POST' }),
  remove: (id: Uuid) =>
    request<void>(`/api/v1/sparks/${id}`, { method: 'DELETE' }),
}

export const runs = {
  list: (status?: RunStatus, signal?: AbortSignal) =>
    request<RunRecord[]>('/api/v1/runs', { query: { status }, signal }),
  get: (id: string, signal?: AbortSignal) =>
    request<RunRecord>(`/api/v1/runs/${id}`, { signal }),
  resume: (id: string) =>
    request<ResumeResponse>(`/api/v1/runs/${id}/resume`, { method: 'POST' }),
  cancel: (id: string) =>
    request<void>(`/api/v1/runs/${id}/cancel`, { method: 'POST' }),
}

export const agent = {
  chat: (body: ChatRequest) =>
    request<ChatResponse>('/api/v1/agent/chat', { method: 'POST', body }),
  status: (signal?: AbortSignal) =>
    request<AgentStatus>('/api/v1/agent/status', { signal }),
}

// Flows (boards) are scoped to an org + optional site, like dashboards. The
// slug-addressed verbs take `?org=` (required) + `?site_id=` (optional, omit =
// the org-level flow of that slug).
type BoardScope = { org: string; siteId?: Uuid }

export const boards = {
  list: (params: BoardScope, signal?: AbortSignal) =>
    request<BoardView[]>('/api/v1/boards', {
      query: { org: params.org, site_id: params.siteId },
      signal,
    }),
  get: (slug: string, scope: BoardScope, signal?: AbortSignal) =>
    request<BoardView>(`/api/v1/boards/${slug}`, {
      query: { org: scope.org, site_id: scope.siteId },
      signal,
    }),
  /** Component catalogue: ports + config schema driving the flow editor. */
  components: (signal?: AbortSignal) =>
    request<ComponentView[]>('/api/v1/boards/components', { signal }),
  /** Create or republish a flow; an edit saves as a new version of the slug. */
  save: (body: CreateBoard) =>
    request<BoardView>('/api/v1/boards', { method: 'POST', body }),
  /** Patch latest-version metadata (`display_name`, `enabled`) of a flow. */
  patch: (slug: string, scope: BoardScope, body: PatchBoard) =>
    request<BoardView>(`/api/v1/boards/${slug}`, {
      method: 'PATCH',
      query: { org: scope.org, site_id: scope.siteId },
      body,
    }),
  remove: (slug: string, scope: BoardScope) =>
    request<void>(`/api/v1/boards/${slug}`, {
      method: 'DELETE',
      query: { org: scope.org, site_id: scope.siteId },
    }),
  runStored: (slug: string, scope: BoardScope) =>
    request<RunBoardResponse>(`/api/v1/boards/${slug}/run`, {
      method: 'POST',
      query: { org: scope.org, site_id: scope.siteId },
    }),
  /** Latest per-node values a running (or last-run) flow produced. */
  outputs: (slug: string, scope: BoardScope, signal?: AbortSignal) =>
    request<PortOutput[]>(`/api/v1/boards/${slug}/outputs`, {
      query: { org: scope.org, site_id: scope.siteId },
      signal,
    }),
  /** Evaluate an inline graph once — runs the live canvas, unsaved edits included. */
  runInline: (board: BoardGraph) =>
    request<RunBoardResponse>('/api/v1/boards/run', {
      method: 'POST',
      body: { board },
    }),
}

export const widgets = {
  list: (params: { siteId?: Uuid; dashboardId?: Uuid }, signal?: AbortSignal) =>
    request<Widget[]>('/api/v1/widgets', {
      query: { site_id: params.siteId, dashboard_id: params.dashboardId },
      signal,
    }),
  get: (id: Uuid, signal?: AbortSignal) =>
    request<Widget>(`/api/v1/widgets/${id}`, { signal }),
  create: (body: CreateWidget) =>
    request<Widget>('/api/v1/widgets', { method: 'POST', body }),
  patch: (id: Uuid, body: PatchWidget) =>
    request<Widget>(`/api/v1/widgets/${id}`, { method: 'PATCH', body }),
  remove: (id: Uuid) =>
    request<void>(`/api/v1/widgets/${id}`, { method: 'DELETE' }),
}

export const dashboards = {
  list: (params: { org: string; siteId?: Uuid }, signal?: AbortSignal) =>
    request<Dashboard[]>('/api/v1/dashboards', {
      query: { org: params.org, site_id: params.siteId },
      signal,
    }),
  get: (id: Uuid, signal?: AbortSignal) =>
    request<Dashboard>(`/api/v1/dashboards/${id}`, { signal }),
  create: (body: CreateDashboard) =>
    request<Dashboard>('/api/v1/dashboards', { method: 'POST', body }),
  patch: (id: Uuid, body: PatchDashboard) =>
    request<Dashboard>(`/api/v1/dashboards/${id}`, { method: 'PATCH', body }),
  remove: (id: Uuid) =>
    request<void>(`/api/v1/dashboards/${id}`, { method: 'DELETE' }),
}

export const query = {
  run: (sql: string) =>
    request<QueryResult>('/api/v1/query', { method: 'POST', body: { sql } }),
}

// Rules are org-owned with an optional site (`?site_id=`); a site rule overrides
// the org-level one of the same name. `list` with a siteId returns that site's
// rules + the org-level ones; without, every rule the org owns.
export const rules = {
  list: (org: string, siteId?: Uuid, signal?: AbortSignal) =>
    request<RuleView[]>(`/api/v1/orgs/${org}/rules`, {
      query: { site_id: siteId },
      signal,
    }),
  get: (org: string, name: string, siteId?: Uuid, signal?: AbortSignal) =>
    request<RuleView>(`/api/v1/orgs/${org}/rules/${name}`, {
      query: { site_id: siteId },
      signal,
    }),
  create: (org: string, body: CreateRule) =>
    request<RuleView>(`/api/v1/orgs/${org}/rules`, { method: 'POST', body }),
  update: (org: string, name: string, siteId: Uuid | undefined, body: UpdateRule) =>
    request<RuleView>(`/api/v1/orgs/${org}/rules/${name}`, {
      method: 'PUT',
      query: { site_id: siteId },
      body,
    }),
  remove: (org: string, name: string, siteId?: Uuid) =>
    request<void>(`/api/v1/orgs/${org}/rules/${name}`, {
      method: 'DELETE',
      query: { site_id: siteId },
    }),
  /** Rules that compose this one — the change-impact / blast-radius list. */
  referencing: (org: string, name: string, siteId?: Uuid, signal?: AbortSignal) =>
    request<RuleView[]>(`/api/v1/orgs/${org}/rules/${name}/referencing`, {
      query: { site_id: siteId },
      signal,
    }),
  /** Run a rule once against a point's history without emitting a spark. */
  dryRun: (org: string, body: DryRunRequest) =>
    request<DryRunResponse>(`/api/v1/orgs/${org}/rules/dry-run`, {
      method: 'POST',
      body,
    }),
}

export const auth = {
  /** The caller's resolved identity + capabilities (permission-aware UI chrome). */
  whoami: (signal?: AbortSignal) =>
    request<Whoami>('/api/v1/whoami', { signal }),
}

// --- RBAC admin surfaces (Members / Teams / Access). Admin-gated server-side. ---

export const users = {
  list: (org: string, signal?: AbortSignal) =>
    request<User[]>(`/api/v1/orgs/${org}/users`, { signal }),
  get: (org: string, id: Uuid, signal?: AbortSignal) =>
    request<User>(`/api/v1/orgs/${org}/users/${id}`, { signal }),
  create: (org: string, body: CreateUser) =>
    request<User>(`/api/v1/orgs/${org}/users`, { method: 'POST', body }),
  patch: (org: string, id: Uuid, body: PatchUser) =>
    request<User>(`/api/v1/orgs/${org}/users/${id}`, { method: 'PATCH', body }),
  remove: (org: string, id: Uuid) =>
    request<void>(`/api/v1/orgs/${org}/users/${id}`, { method: 'DELETE' }),
}

export const teams = {
  list: (org: string, signal?: AbortSignal) =>
    request<Team[]>(`/api/v1/orgs/${org}/teams`, { signal }),
  get: (org: string, id: Uuid, signal?: AbortSignal) =>
    request<Team>(`/api/v1/orgs/${org}/teams/${id}`, { signal }),
  create: (org: string, body: CreateTeam) =>
    request<Team>(`/api/v1/orgs/${org}/teams`, { method: 'POST', body }),
  patch: (org: string, id: Uuid, body: PatchTeam) =>
    request<Team>(`/api/v1/orgs/${org}/teams/${id}`, { method: 'PATCH', body }),
  remove: (org: string, id: Uuid) =>
    request<void>(`/api/v1/orgs/${org}/teams/${id}`, { method: 'DELETE' }),
  members: (org: string, id: Uuid, signal?: AbortSignal) =>
    request<User[]>(`/api/v1/orgs/${org}/teams/${id}/members`, { signal }),
  addMember: (org: string, id: Uuid, userId: Uuid) =>
    request<void>(`/api/v1/orgs/${org}/teams/${id}/members`, {
      method: 'POST',
      body: { user_id: userId },
    }),
  removeMember: (org: string, id: Uuid, userId: Uuid) =>
    request<void>(`/api/v1/orgs/${org}/teams/${id}/members/${userId}`, {
      method: 'DELETE',
    }),
}

export const grants = {
  list: (org: string, resourceRef?: string, signal?: AbortSignal) =>
    request<Grant[]>(`/api/v1/orgs/${org}/grants`, {
      query: { resource_ref: resourceRef },
      signal,
    }),
  create: (org: string, body: CreateGrant) =>
    request<Grant>(`/api/v1/orgs/${org}/grants`, { method: 'POST', body }),
  remove: (org: string, id: Uuid) =>
    request<void>(`/api/v1/orgs/${org}/grants/${id}`, { method: 'DELETE' }),
  forDashboard: (id: Uuid, signal?: AbortSignal) =>
    request<Grant[]>(`/api/v1/dashboards/${id}/grants`, { signal }),
  grantDashboard: (id: Uuid, body: CreateDashboardGrant) =>
    request<Grant>(`/api/v1/dashboards/${id}/grants`, { method: 'POST', body }),
}
