/**
 * React Query hooks over the endpoint functions. Reads poll on an interval so
 * live `cur` values and new sparks surface without a manual refresh; mutations
 * invalidate the affected keys.
 */
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import * as api from './endpoints';
import { qk } from './keys';
import type { CreateWidget, CurRequest, Uuid, WriteRequest } from './types';

const LIVE_INTERVAL = 5_000;

export function useSites() {
  return useQuery({ queryKey: qk.sites, queryFn: ({ signal }) => api.sites.list(signal) });
}

export function useEquips(siteId?: Uuid) {
  return useQuery({
    queryKey: qk.equips(siteId),
    queryFn: ({ signal }) => api.equips.list(siteId, signal),
  });
}

export function usePoints(params: { equipId?: Uuid; siteId?: Uuid; tags?: string }) {
  return useQuery({
    queryKey: qk.points(params),
    queryFn: ({ signal }) => api.points.list(params, signal),
    refetchInterval: LIVE_INTERVAL,
  });
}

export function usePointHistory(id: Uuid | undefined) {
  return useQuery({
    queryKey: qk.pointHistory(id ?? 'none'),
    queryFn: ({ signal }) => api.points.history(id as Uuid, signal),
    enabled: Boolean(id),
    refetchInterval: LIVE_INTERVAL,
  });
}

export function useSparks(siteId?: Uuid) {
  return useQuery({
    queryKey: qk.sparks(siteId),
    queryFn: ({ signal }) => api.sparks.list(siteId, signal),
    refetchInterval: LIVE_INTERVAL,
  });
}

/** Poll the run list while any run is suspended so the approval queue stays live. */
export function useRuns() {
  return useQuery({
    queryKey: qk.runs,
    queryFn: ({ signal }) => api.runs.list(signal),
    refetchInterval: (query) =>
      (query.state.data ?? []).some((r) => r.status === 'suspended') ? LIVE_INTERVAL : false,
  });
}

/** One run; polls while suspended so an approval landing elsewhere reflects here. */
export function useRun(id: string | undefined) {
  return useQuery({
    queryKey: qk.run(id ?? 'none'),
    queryFn: ({ signal }) => api.runs.get(id as string, signal),
    enabled: Boolean(id),
    refetchInterval: (query) => (query.state.data?.status === 'suspended' ? LIVE_INTERVAL : false),
  });
}

/**
 * Approve a suspended run: the agent's held write is re-applied through the
 * priority array. Invalidate runs and points so the resumed status and the
 * agent's write at its priority slot both surface.
 */
export function useResumeRun() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => api.runs.resume(id),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['runs'] });
      qc.invalidateQueries({ queryKey: ['points'] });
    },
  });
}

/** Reject a suspended run: the held write is discarded; refresh the run state. */
export function useCancelRun() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => api.runs.cancel(id),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['runs'] }),
  });
}

export function useAckSpark(siteId?: Uuid) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: Uuid) => api.sparks.ack(id),
    onSuccess: () => qc.invalidateQueries({ queryKey: qk.sparks(siteId) }),
  });
}

export function useWritePoint() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ id, body }: { id: Uuid; body: WriteRequest }) => api.points.write(id, body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['points'] }),
  });
}

export function useRelinquishPoint() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ id, priority }: { id: Uuid; priority: number }) =>
      api.points.relinquish(id, priority),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['points'] }),
  });
}

export function useIngestPoint() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ id, body }: { id: Uuid; body: CurRequest }) => api.points.ingest(id, body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['points'] }),
  });
}

export function useAgentChat() {
  return useMutation({ mutationFn: api.agent.chat });
}

export function useBoards() {
  return useQuery({ queryKey: qk.boards, queryFn: ({ signal }) => api.boards.list(signal) });
}

/** Run a stored board on demand; resolves the run's outport packets. */
export function useRunStoredBoard() {
  return useMutation({ mutationFn: (slug: string) => api.boards.runStored(slug) });
}

/** Pinned dashboard widgets for a site; polls so live tiles stay fresh. */
export function useWidgets(siteId?: Uuid) {
  return useQuery({
    queryKey: qk.widgets(siteId),
    queryFn: ({ signal }) => api.widgets.list(siteId, signal),
    refetchInterval: LIVE_INTERVAL,
  });
}

/** Pin a widget; invalidate the site's widget list so it appears on the canvas. */
export function useCreateWidget(siteId?: Uuid) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (body: CreateWidget) => api.widgets.create(body),
    onSuccess: () => qc.invalidateQueries({ queryKey: qk.widgets(siteId) }),
  });
}
