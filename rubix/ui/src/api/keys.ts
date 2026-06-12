/** React Query key factory — central so invalidation stays consistent. */
import type { Uuid } from './types';

export const qk = {
  sites: ['sites'] as const,
  site: (id: Uuid) => ['sites', id] as const,
  equips: (siteId?: Uuid) => ['equips', siteId ?? 'all'] as const,
  points: (params: { equipId?: Uuid; siteId?: Uuid; tags?: string }) =>
    ['points', params.equipId ?? null, params.siteId ?? null, params.tags ?? null] as const,
  point: (id: Uuid) => ['points', 'one', id] as const,
  pointHistory: (id: Uuid) => ['points', id, 'history'] as const,
  sparks: (siteId?: Uuid) => ['sparks', siteId ?? 'all'] as const,
  runs: ['runs'] as const,
  run: (id: string) => ['runs', id] as const,
  boards: ['boards'] as const,
  board: (slug: string) => ['boards', slug] as const,
  widgets: (siteId?: Uuid) => ['widgets', siteId ?? 'all'] as const,
};
