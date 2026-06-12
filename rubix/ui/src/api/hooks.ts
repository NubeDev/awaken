/**
 * React Query hooks over the endpoint functions. Reads poll on an interval so
 * live `cur` values and new sparks surface without a manual refresh; mutations
 * invalidate the affected keys.
 */
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import * as api from './endpoints';
import { qk } from './keys';
import type { CurRequest, Uuid, WriteRequest } from './types';

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

export function useRuns() {
  return useQuery({ queryKey: qk.runs, queryFn: ({ signal }) => api.runs.list(signal) });
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
