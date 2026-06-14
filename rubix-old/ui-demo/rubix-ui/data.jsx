/* RUBIX — mock domain data (BMS/EMS) */

/* deterministic pseudo-random so charts are stable across renders */
function rng(seed) { let s = seed % 2147483647; if (s <= 0) s += 2147483646; return () => (s = s * 16807 % 2147483647) / 2147483647; }

/* build a time-series: n points, base value, amplitude, seed, trend */
function series(n, base, amp, seed, { trend = 0, noise = 0.4, period = 24 } = {}) {
  const r = rng(seed); const out = [];
  for (let i = 0; i < n; i++) {
    const wave = Math.sin((i / period) * Math.PI * 2) * amp;
    const wobble = (r() - 0.5) * amp * noise;
    out.push(+(base + wave + wobble + trend * i).toFixed(2));
  }
  return out;
}

const SITES = [
  { id: 's1', org: 'acme', slug: 'hq-tower', name: 'HQ Tower', city: 'Sydney', equips: 142, points: 3870, online: true, kw: 412, eui: 138, alarms: 3 },
  { id: 's2', org: 'acme', slug: 'distribution-w', name: 'Distribution West', city: 'Perth', equips: 88, points: 2110, online: true, kw: 286, eui: 96, alarms: 1 },
  { id: 's3', org: 'acme', slug: 'lab-campus', name: 'Lab Campus', city: 'Melbourne', equips: 210, points: 6240, online: true, kw: 738, eui: 204, alarms: 5 },
  { id: 's4', org: 'acme', slug: 'cold-store-3', name: 'Cold Store 3', city: 'Brisbane', equips: 54, points: 1290, online: false, kw: 0, eui: 0, alarms: 2 },
];

/* equipment tree for active site (HQ Tower) */
const EQUIPS = [
  { id: 'e1', path: 'ahu-1', name: 'AHU-1 · L1 East', kind: 'ahu', tags: ['ahu', 'discharge', 'hvac'], online: true, points: 38, alarm: false, icon: 'fan' },
  { id: 'e2', path: 'ahu-3', name: 'AHU-3 · L4 West', kind: 'ahu', tags: ['ahu', 'discharge', 'hvac'], online: true, points: 38, alarm: true, icon: 'fan' },
  { id: 'e3', path: 'chiller-1', name: 'Chiller-1', kind: 'chiller', tags: ['chiller', 'cool', 'plant'], online: true, points: 52, alarm: false, icon: 'droplet' },
  { id: 'e4', path: 'chiller-2', name: 'Chiller-2', kind: 'chiller', tags: ['chiller', 'cool', 'plant'], online: true, points: 52, alarm: false, icon: 'droplet' },
  { id: 'e5', path: 'boiler-1', name: 'Boiler-1', kind: 'boiler', tags: ['boiler', 'heat', 'plant'], online: true, points: 30, alarm: false, icon: 'flame' },
  { id: 'e6', path: 'meter-main', name: 'Main Incomer', kind: 'meter', tags: ['elec', 'meter', 'energy'], online: true, points: 24, alarm: false, icon: 'zap' },
  { id: 'e7', path: 'vav-4-12', name: 'VAV 4-12', kind: 'vav', tags: ['vav', 'zone', 'hvac'], online: true, points: 18, alarm: false, icon: 'wind' },
  { id: 'e8', path: 'vav-4-13', name: 'VAV 4-13', kind: 'vav', tags: ['vav', 'zone', 'hvac'], online: true, points: 18, alarm: false, icon: 'wind' },
  { id: 'e9', path: 'ct-1', name: 'Cooling Tower 1', kind: 'tower', tags: ['tower', 'cool', 'plant'], online: true, points: 22, alarm: false, icon: 'wind' },
];

/* points for AHU-3 (the one with the alarm) — drives the priority-array detail */
function pa(slots, def) { const a = new Array(16).fill(null); slots.forEach(([lvl, v, who]) => a[lvl - 1] = { v, who }); return { slots: a, def }; }

const POINTS = [
  { id: 'p1', equip: 'e2', slug: 'discharge-temp', name: 'Discharge Air Temp', kind: 'sensor', unit: '°C', tags: ['discharge', 'air', 'temp', 'sensor'], cur: 13.4, ts: '12s', seed: 11, base: 13.5, amp: 1.4, status: 'ok' },
  { id: 'p2', equip: 'e2', slug: 'return-temp', name: 'Return Air Temp', kind: 'sensor', unit: '°C', tags: ['return', 'air', 'temp', 'sensor'], cur: 22.8, ts: '12s', seed: 21, base: 22.6, amp: 0.8, status: 'ok' },
  { id: 'p3', equip: 'e2', slug: 'supply-fan-cmd', name: 'Supply Fan Speed', kind: 'cmd', unit: '%', tags: ['supply', 'fan', 'cmd'], cur: 82, ts: '4s', seed: 31, base: 78, amp: 8, status: 'ok',
    pa: pa([[8, 82, 'operator'], [13, 70, 'awaken·agent'], [16, 60, 'schedule']], 0) },
  { id: 'p4', equip: 'e2', slug: 'cooling-valve', name: 'Cooling Valve', kind: 'cmd', unit: '%', tags: ['cool', 'valve', 'cmd'], cur: 96, ts: '4s', seed: 41, base: 60, amp: 30, status: 'fault',
    pa: pa([[13, 96, 'awaken·agent'], [16, 40, 'schedule']], 0) },
  { id: 'p5', equip: 'e2', slug: 'heating-valve', name: 'Heating Valve', kind: 'cmd', unit: '%', tags: ['heat', 'valve', 'cmd'], cur: 35, ts: '4s', seed: 51, base: 20, amp: 18, status: 'fault',
    pa: pa([[16, 35, 'schedule']], 0) },
  { id: 'p6', equip: 'e2', slug: 'discharge-sp', name: 'Discharge Temp Setpoint', kind: 'sp', unit: '°C', tags: ['discharge', 'temp', 'sp'], cur: 13.0, ts: '1m', seed: 61, base: 13, amp: 0.3, status: 'ok',
    pa: pa([[10, 13.0, 'operator'], [16, 14.0, 'schedule']], 14.0) },
  { id: 'p7', equip: 'e2', slug: 'occupancy', name: 'Zone Occupancy', kind: 'sensor', unit: '', tags: ['zone', 'occ', 'sensor'], cur: 'Occupied', ts: '30s', seed: 71, base: 1, amp: 0, status: 'ok' },
  { id: 'p8', equip: 'e2', slug: 'static-press', name: 'Duct Static Pressure', kind: 'sensor', unit: 'Pa', tags: ['duct', 'pressure', 'sensor'], cur: 248, ts: '4s', seed: 81, base: 250, amp: 14, status: 'ok' },
];

const SPARKS = [
  { id: 'sp1', rule: 'simultaneous-heat-cool', severity: 'fault', equip: 'AHU-3', site: 'HQ Tower', points: ['cooling-valve', 'heating-valve'], message: 'Simultaneous heating and cooling — cooling valve 96% while heating valve 35%', ts: '6 min ago', ack: false, agent: true },
  { id: 'sp2', rule: 'rogue-zone', severity: 'fault', equip: 'Chiller-2', site: 'Lab Campus', points: ['chw-supply-temp'], message: 'CHW supply temp 9.2°C above setpoint for 14 min — possible fouling', ts: '22 min ago', ack: false, agent: false },
  { id: 'sp3', rule: 'stuck-damper', severity: 'warning', equip: 'VAV 4-12', site: 'HQ Tower', points: ['damper-pos'], message: 'Damper command changed 40% but airflow flat — possible stuck actuator', ts: '38 min ago', ack: false, agent: false },
  { id: 'sp4', rule: 'after-hours-runtime', severity: 'warning', equip: 'AHU-1', site: 'HQ Tower', points: ['supply-fan-cmd'], message: 'Fan running 3.2h after scheduled off — 41 kWh waste', ts: '1 hr ago', ack: false, agent: false },
  { id: 'sp5', rule: 'sensor-drift', severity: 'info', equip: 'Boiler-1', site: 'HQ Tower', points: ['flue-temp'], message: 'Flue temp sensor drift detected vs sibling sensor (1.8°C)', ts: '2 hr ago', ack: true, agent: false },
  { id: 'sp6', rule: 'demand-spike', severity: 'warning', equip: 'Main Incomer', site: 'Distribution West', points: ['kw-total'], message: 'Peak demand approaching 92% of contracted capacity', ts: '3 hr ago', ack: false, agent: false },
  { id: 'sp7', rule: 'low-delta-t', severity: 'info', equip: 'Chiller-1', site: 'HQ Tower', points: ['chw-dt'], message: 'Chilled water ΔT 3.1°C — below 5°C design (low-ΔT syndrome)', ts: '5 hr ago', ack: true, agent: false },
];

/* agent run / chat transcript for the AHU-3 finding */
const AGENT_RUN = {
  id: 'run_8fa2', title: 'Diagnose AHU-3 simultaneous heat/cool', status: 'awaiting-approval', trigger: 'spark', started: '6 min ago', site: 'HQ Tower',
  steps: [
    { type: 'trigger', label: 'Spark finding', text: 'simultaneous-heat-cool on AHU-3 (fault)' },
    { type: 'thinking', text: 'A fault arrived for AHU-3. I’ll pull the live valve commands and recent discharge-temp history before proposing a fix.' },
    { type: 'tool', tool: 'read_point', args: 'acme/hq-tower/ahu-3/cooling-valve/cur', result: '96 %', ok: true },
    { type: 'tool', tool: 'read_point', args: 'acme/hq-tower/ahu-3/heating-valve/cur', result: '35 %', ok: true },
    { type: 'tool', tool: 'query', args: "SELECT ts, val FROM his WHERE point='ahu-3/discharge-temp' ORDER BY ts DESC LIMIT 60", result: '60 rows · trend rising 1.9°C/30min', ok: true },
    { type: 'finding', text: 'The heating valve is being driven by a stale schedule slot (priority 16) while the agent-level cooling command sits at priority 13. Net effect: the coil pair is fighting, wasting ~2.4 kW and pushing discharge temp up.' },
    { type: 'proposal', text: 'Relinquish the heating valve schedule slot and cap cooling valve to 70% at agent priority. Discharge temp should settle to the 13.0°C setpoint within ~8 min.',
      writes: [
        { point: 'ahu-3/heating-valve', action: 'relinquish', level: 16, from: '35%', to: '0% (off)', priority: 13 },
        { point: 'ahu-3/cooling-valve', action: 'write', level: 13, from: '96%', to: '70%', priority: 13 },
      ] },
  ],
};

/* energy KPI history (48h hourly) */
const ENERGY = {
  demand: series(48, 360, 120, 7, { period: 24, noise: 0.25 }),
  baseline: series(48, 340, 90, 9, { period: 24, noise: 0.15 }),
  today: series(48, 138, 30, 3, { period: 24, noise: 0.3 }),
};

/* per-system load split */
const LOAD_SPLIT = [
  { label: 'Chillers', kw: 168, color: 'var(--chart-1)' },
  { label: 'AHUs / Fans', kw: 96, color: 'var(--chart-2)' },
  { label: 'Lighting', kw: 64, color: 'var(--chart-3)' },
  { label: 'Plug loads', kw: 52, color: 'var(--chart-4)' },
  { label: 'Other', kw: 32, color: 'var(--chart-5)' },
];

/* dashboards (builder) */
const DASHBOARDS = [
  { id: 'd1', name: 'Site Overview', widgets: 8, updated: '2h ago', owner: 'You', shared: true },
  { id: 'd2', name: 'Energy & Demand', widgets: 6, updated: 'yesterday', owner: 'You', shared: true },
  { id: 'd3', name: 'Chiller Plant', widgets: 11, updated: '3d ago', owner: 'M. Tan', shared: false },
  { id: 'd4', name: 'AHU Fleet Health', widgets: 9, updated: '1w ago', owner: 'awaken', shared: true },
];

const NAV = [
  { id: 'dashboard', label: 'Dashboard', icon: 'gauge' },
  { id: 'builder', label: 'Dashboard Builder', icon: 'layout-dashboard' },
  { id: 'sparks', label: 'Sparks', icon: 'zap', badge: 'sparks' },
  { id: 'points', label: 'Points & Equip', icon: 'network' },
  { id: 'flows', label: 'Flow Boards', icon: 'workflow' },
];
const NAV2 = [
  { id: 'history', label: 'History & SQL', icon: 'database' },
  { id: 'runs', label: 'Agent Runs', icon: 'sparkles' },
];

/* board nodes for the flow canvas (reflow / react-flow style) */
const FLOW = {
  name: 'AHU-3 · Discharge Reset',
  nodes: [
    { id: 'n1', type: 'source', title: 'Zone Occupancy', sub: 'subscribe', x: 40, y: 90, kind: 'in', icon: 'circle-dot', outs: ['occ'] },
    { id: 'n2', type: 'source', title: 'Outside Air Temp', sub: 'subscribe', x: 40, y: 230, kind: 'in', icon: 'thermometer', outs: ['oat'] },
    { id: 'n3', type: 'logic', title: 'Schedule', sub: 'occupied → 13.0°C', x: 320, y: 90, kind: 'logic', icon: 'calendar', ins: ['occ'], outs: ['sp'] },
    { id: 'n4', type: 'logic', title: 'Reset Curve', sub: 'OAT reset 12–16°C', x: 320, y: 250, kind: 'logic', icon: 'function', ins: ['oat'], outs: ['reset'] },
    { id: 'n5', type: 'logic', title: 'PID', sub: 'discharge control', x: 600, y: 160, kind: 'logic', icon: 'route', ins: ['sp', 'reset', 'pv'], outs: ['out'] },
    { id: 'n6', type: 'sensor', title: 'Discharge Temp', sub: 'read_point', x: 320, y: 410, kind: 'in', icon: 'thermometer', outs: ['pv'] },
    { id: 'n7', type: 'write', title: 'Cooling Valve', sub: 'write · prio 13', x: 880, y: 110, kind: 'out', icon: 'droplet', ins: ['cmd'] },
    { id: 'n8', type: 'agent', title: 'awaken · guard', sub: 'agent_call', x: 880, y: 250, kind: 'agent', icon: 'sparkles', ins: ['cmd'], outs: ['ok'] },
  ],
  edges: [
    ['n1', 'n3'], ['n2', 'n4'], ['n3', 'n5'], ['n4', 'n5'], ['n6', 'n5'], ['n5', 'n7'], ['n5', 'n8'],
  ],
};

/* node palette for flow editor */
const NODE_PALETTE = [
  { group: 'Sources', items: [{ icon: 'circle-dot', label: 'Point Read' }, { icon: 'history', label: 'History Query' }, { icon: 'calendar', label: 'Schedule' }, { icon: 'clock', label: 'Interval' }] },
  { group: 'Logic', items: [{ icon: 'function', label: 'Reset Curve' }, { icon: 'route', label: 'PID' }, { icon: 'sigma', label: 'Math' }, { icon: 'git-branch', label: 'Compare' }] },
  { group: 'AI', items: [{ icon: 'sparkles', label: 'Agent Call' }, { icon: 'shield-check', label: 'HITL Gate' }] },
  { group: 'Sinks', items: [{ icon: 'droplet', label: 'Point Write' }, { icon: 'zap', label: 'Emit Finding' }, { icon: 'pin', label: 'Pin Widget' }] },
];

Object.assign(window, {
  series, SITES, EQUIPS, POINTS, SPARKS, AGENT_RUN, ENERGY, LOAD_SPLIT,
  DASHBOARDS, NAV, NAV2, FLOW, NODE_PALETTE,
});
