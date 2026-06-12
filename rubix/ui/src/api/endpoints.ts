/**
 * Typed endpoint functions, one per `/api/v1/*` route the UI consumes. Each is
 * a pure call into `request()`; React Query hooks wrap these for caching.
 *
 * When demo mode is on (`isDemo()`), reads/writes resolve from in-memory
 * fixtures instead of the network, so the UI is fully populated without a
 * backend. Real mode is unaffected — set `VITE_DEMO=0` to force the live API.
 */
import { request } from './client';
import { demo, isDemo } from './demo';
import type {
  ChatRequest,
  ChatResponse,
  CurRequest,
  Equip,
  HisSample,
  Point,
  PointEnvelope,
  QueryResult,
  RunSummary,
  Site,
  Spark,
  Uuid,
  WriteRequest,
} from './types';

export const sites = {
  list: (signal?: AbortSignal) =>
    isDemo() ? demo.sites.list() : request<Site[]>('/api/v1/sites', { signal }),
  get: (id: Uuid, signal?: AbortSignal) => request<Site>(`/api/v1/sites/${id}`, { signal }),
};

export const equips = {
  list: (siteId?: Uuid, signal?: AbortSignal) =>
    isDemo()
      ? demo.equips.list(siteId)
      : request<Equip[]>('/api/v1/equips', { query: { site_id: siteId }, signal }),
  get: (id: Uuid, signal?: AbortSignal) => request<Equip>(`/api/v1/equips/${id}`, { signal }),
};

export const points = {
  list: (params: { equipId?: Uuid; siteId?: Uuid; tags?: string }, signal?: AbortSignal) =>
    isDemo()
      ? demo.points.list(params)
      : request<Point[]>('/api/v1/points', {
          query: { equip_id: params.equipId, site_id: params.siteId, tags: params.tags },
          signal,
        }),
  get: (id: Uuid, signal?: AbortSignal) => request<PointEnvelope>(`/api/v1/points/${id}`, { signal }),
  history: (id: Uuid, signal?: AbortSignal) =>
    isDemo() ? demo.points.history(id) : request<HisSample[]>(`/api/v1/points/${id}/his`, { signal }),
  write: (id: Uuid, body: WriteRequest) =>
    isDemo()
      ? demo.points.write(id, body)
      : request<PointEnvelope>(`/api/v1/points/${id}/write`, { method: 'POST', body }),
  relinquish: (id: Uuid, priority: number) =>
    isDemo()
      ? demo.points.relinquish(id, priority)
      : request<PointEnvelope>(`/api/v1/points/${id}/write/${priority}`, { method: 'DELETE' }),
  ingest: (id: Uuid, body: CurRequest) =>
    isDemo()
      ? demo.points.ingest(id, body)
      : request<PointEnvelope>(`/api/v1/points/${id}/cur`, { method: 'POST', body }),
};

export const sparks = {
  list: (siteId?: Uuid, signal?: AbortSignal) =>
    isDemo()
      ? demo.sparks.list(siteId)
      : request<Spark[]>('/api/v1/sparks', { query: { site_id: siteId }, signal }),
  ack: (id: Uuid) =>
    isDemo() ? demo.sparks.ack(id) : request<Spark>(`/api/v1/sparks/${id}/ack`, { method: 'POST' }),
};

export const runs = {
  list: (signal?: AbortSignal) =>
    isDemo() ? demo.runs.list() : request<RunSummary[]>('/api/v1/runs', { signal }),
  get: (id: string, signal?: AbortSignal) => request<RunSummary>(`/api/v1/runs/${id}`, { signal }),
  resume: (id: string) => request<RunSummary>(`/api/v1/runs/${id}/resume`, { method: 'POST' }),
  cancel: (id: string) => request<RunSummary>(`/api/v1/runs/${id}/cancel`, { method: 'POST' }),
};

export const agent = {
  chat: (body: ChatRequest) =>
    isDemo()
      ? demo.agent.chat()
      : request<ChatResponse>('/api/v1/agent/chat', { method: 'POST', body }),
};

export const query = {
  run: (sql: string) =>
    isDemo() ? demo.query.run() : request<QueryResult>('/api/v1/query', { method: 'POST', body: { sql } }),
};
