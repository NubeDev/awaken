import { describe, expect, it } from 'vitest';
import type { Equip, Point, Site } from './types';
import { keyexprIndex, pointKeyexpr } from './keyexpr';

const site = {
  id: 's1',
  org: 'acme',
  slug: 'hq',
  display_name: 'HQ',
  tags: {},
  created_at: '2026-01-01T00:00:00Z',
} satisfies Site;

const equip = {
  id: 'e1',
  site_id: 's1',
  path: 'ahu-3',
  display_name: 'AHU 3',
  tags: {},
  created_at: '2026-01-01T00:00:00Z',
} satisfies Equip;

const point = {
  id: 'p1',
  equip_id: 'e1',
  slug: 'sat',
  display_name: 'Supply Air Temp',
  kind: 'sensor',
  unit: '°C',
  tags: {},
  priority_array: { slots: [], relinquish_default: null },
  cur_value: 21.4,
  cur_ts: '2026-01-01T00:00:00Z',
  created_at: '2026-01-01T00:00:00Z',
} satisfies Point;

describe('pointKeyexpr', () => {
  it('builds the server keyexpr without a /cur suffix', () => {
    expect(pointKeyexpr(site, equip, point)).toBe('acme/hq/ahu-3/sat');
  });
});

describe('keyexprIndex', () => {
  it('resolves a widget target keyexpr back to its point', () => {
    const index = keyexprIndex(site, [equip], [point]);
    expect(index.get('acme/hq/ahu-3/sat')).toBe(point);
  });

  it('skips points whose equip is absent rather than guessing a path', () => {
    const orphan = { ...point, id: 'p2', equip_id: 'missing' } satisfies Point;
    const index = keyexprIndex(site, [equip], [point, orphan]);
    expect(index.size).toBe(1);
    expect(index.get('acme/hq/ahu-3/sat')).toBe(point);
  });
});
