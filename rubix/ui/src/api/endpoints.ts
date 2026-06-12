/**
 * Typed endpoint functions, one per `/api/v1/*` route the UI consumes. Each is
 * a pure call into `request()`; React Query hooks wrap these for caching.
 */
import { request } from './client';
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
  list: (signal?: AbortSignal) => request<Site[]>('/api/v1/sites', { signal }),
  get: (id: Uuid, signal?: AbortSignal) => request<Site>(`/api/v1/sites/${id}`, { signal }),
};

export const equips = {
  list: (siteId?: Uuid, signal?: AbortSignal) =>
    request<Equip[]>('/api/v1/equips', { query: { site_id: siteId }, signal }),
  get: (id: Uuid, signal?: AbortSignal) => request<Equip>(`/api/v1/equips/${id}`, { signal }),
};

export const points = {
  list: (params: { equipId?: Uuid; siteId?: Uuid; tags?: string }, signal?: AbortSignal) =>
    request<Point[]>('/api/v1/points', {
      query: { equip_id: params.equipId, site_id: params.siteId, tags: params.tags },
      signal,
    }),
  get: (id: Uuid, signal?: AbortSignal) => request<PointEnvelope>(`/api/v1/points/${id}`, { signal }),
  history: (id: Uuid, signal?: AbortSignal) =>
    request<HisSample[]>(`/api/v1/points/${id}/his`, { signal }),
  write: (id: Uuid, body: WriteRequest) =>
    request<PointEnvelope>(`/api/v1/points/${id}/write`, { method: 'POST', body }),
  relinquish: (id: Uuid, priority: number) =>
    request<PointEnvelope>(`/api/v1/points/${id}/write/${priority}`, { method: 'DELETE' }),
  ingest: (id: Uuid, body: CurRequest) =>
    request<PointEnvelope>(`/api/v1/points/${id}/cur`, { method: 'POST', body }),
};

export const sparks = {
  list: (siteId?: Uuid, signal?: AbortSignal) =>
    request<Spark[]>('/api/v1/sparks', { query: { site_id: siteId }, signal }),
  ack: (id: Uuid) => request<Spark>(`/api/v1/sparks/${id}/ack`, { method: 'POST' }),
};

export const runs = {
  list: (signal?: AbortSignal) => request<RunSummary[]>('/api/v1/runs', { signal }),
  get: (id: string, signal?: AbortSignal) => request<RunSummary>(`/api/v1/runs/${id}`, { signal }),
  resume: (id: string) => request<RunSummary>(`/api/v1/runs/${id}/resume`, { method: 'POST' }),
  cancel: (id: string) => request<RunSummary>(`/api/v1/runs/${id}/cancel`, { method: 'POST' }),
};

export const agent = {
  chat: (body: ChatRequest) => request<ChatResponse>('/api/v1/agent/chat', { method: 'POST', body }),
};

export const query = {
  run: (sql: string) => request<QueryResult>('/api/v1/query', { method: 'POST', body: { sql } }),
};
