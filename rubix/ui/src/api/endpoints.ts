/**
 * Typed endpoint functions, one per `/api/v1/*` route the UI consumes. Each is
 * a pure call into `request()`; React Query hooks wrap these for caching. The
 * only data source is the live rubix-server API — there is no fixture path.
 */
import { request } from './client'
import type {
  BoardGraph,
  BoardView,
  ComponentView,
  CreateBoard,
  CreateEquip,
  CreatePoint,
  CreateSite,
  ChatRequest,
  ChatResponse,
  CurRequest,
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
  Dashboard,
  PatchDashboard,
  ResumeResponse,
  RunBoardResponse,
  RunRecord,
  Site,
  Spark,
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
  list: (signal?: AbortSignal) =>
    request<RunRecord[]>('/api/v1/runs', { signal }),
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
}

export const boards = {
  list: (signal?: AbortSignal) =>
    request<BoardView[]>('/api/v1/boards', { signal }),
  get: (slug: string, signal?: AbortSignal) =>
    request<BoardView>(`/api/v1/boards/${slug}`, { signal }),
  /** Component catalogue: ports + config schema driving the flow editor. */
  components: (signal?: AbortSignal) =>
    request<ComponentView[]>('/api/v1/boards/components', { signal }),
  /** Create or republish a board; an edit saves as a new version of the slug. */
  save: (body: CreateBoard) =>
    request<BoardView>('/api/v1/boards', { method: 'POST', body }),
  /** Patch latest-version metadata (`display_name`, `enabled`) of a board slug. */
  patch: (slug: string, body: PatchBoard) =>
    request<BoardView>(`/api/v1/boards/${slug}`, { method: 'PATCH', body }),
  remove: (slug: string) =>
    request<void>(`/api/v1/boards/${slug}`, { method: 'DELETE' }),
  runStored: (slug: string) =>
    request<RunBoardResponse>(`/api/v1/boards/${slug}/run`, { method: 'POST' }),
  /** Latest per-node values a running (or last-run) board produced. */
  outputs: (slug: string, signal?: AbortSignal) =>
    request<PortOutput[]>(`/api/v1/boards/${slug}/outputs`, { signal }),
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
