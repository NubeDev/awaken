// ============ Rubix Copilot · DATA ============
window.RX = window.RX || {};

RX.site = { name:'Sydney HQ', tz:'AEST', points:312, time:'14:18' };

RX.vitals = [
  { id:'energy', label:'Energy', value:'1,284', unit:'kWh', delta:'-4.2%', good:true },
  { id:'demand', label:'Demand', value:'86.4', unit:'kW', delta:'86% cap', good:false },
  { id:'comfort', label:'Comfort', value:'7/8', unit:'zones', delta:'1 alert', good:false },
  { id:'solar', label:'Solar', value:'12.1', unit:'kW', delta:'14% load', good:true, accent:'amber' }
];

// time series (24 hourly points, 0..23) + forecast tail
RX.series = {
  demand:  [42,39,37,36,37,41,52,68,82,88,90,89,87,88,91,89,86,83,78,70,61,53,47,43],
  demandForecast:[86,89,92,94,93,90,86,82], // from 14:00 onward projection (8 pts)
  precooled:[86,87,88,88,87,85,82,79],      // after pre-cool applied
  solar:   [0,0,0,0,1,4,9,16,24,30,34,36,35,32,28,23,17,11,5,1,0,0,0,0],
  serverTemp:[18.1,18.0,18.2,18.1,19.0,20.4,21.8,22.9,23.6,24.1,24.6],
  serverRecover:[24.6,24.2,23.4,22.3,21.0,19.8,18.9,18.3,18.1], // after failover
  night:   [42,40,38,37,36,37,39,41,40,42,44,43],
  weekThis:[178,165,159,171,168,142,131],
  weekLast:[190,182,176,184,180,150,138]
};

// demand driver breakdown (live)
RX.drivers = [
  { label:'HVAC',      kw:46.6, pct:54, color:'amber' },
  { label:'Lighting',  kw:19.9, pct:23, color:'cool' },
  { label:'Plug load', kw:12.1, pct:14, color:'r1' },
  { label:'Lifts',     kw:5.2,  pct:6,  color:'r2' },
  { label:'Other',     kw:2.6,  pct:3,  color:'muted' }
];

// HVAC zones (the live building)
RX.zones = [
  { id:'l6', name:'Level 6 · Exec',   sp:22.5, temp:22.7, rh:46, load:9.2,  sev:'green', occ:'Low' },
  { id:'l5', name:'Level 5 · Server', sp:18.0, temp:24.6, rh:38, load:21.4, sev:'crit',  occ:'—', note:'CRAC-2 fault' },
  { id:'l4', name:'Level 4 · West',   sp:22.0, temp:23.9, rh:44, load:14.8, sev:'amber', occ:'High' },
  { id:'l3', name:'Level 3 · East',   sp:22.0, temp:21.8, rh:47, load:13.1, sev:'green', occ:'High' },
  { id:'l2', name:'Level 2 · West',   sp:22.0, temp:null, rh:null,load:0,    sev:'muted', occ:'—', note:'AHU offline' },
  { id:'l1', name:'Level 1 · North',  sp:22.0, temp:22.1, rh:49, load:11.6, sev:'green', occ:'Med' },
  { id:'lobby', name:'Lobby',         sp:23.0, temp:22.9, rh:51, load:7.4,  sev:'green', occ:'Med' },
  { id:'cp', name:'Carpark',          sp:20.0, temp:19.7, rh:55, load:4.9,  sev:'green', occ:'Low' }
];

// the attention queue — what Rubix wants you to see, ranked
RX.moments = [
  { id:'server', sev:'crit', icon:'thermometer-sun', time:'14:12', src:'Rule · Critical temp',
    title:'Server room is overheating',
    say:'CRAC-2\u2019s compressor faulted four minutes ago. The Level 5 rack inlet is at 24.6° and still climbing — 6.6° above its 18° setpoint. I\u2019ve dispatched a technician and staged the backup unit, but you may want to move non-critical load off this floor now.',
    viz:'temp',
    stats:[['Current','24.6°','crit'],['Setpoint','18.0°',''],['Deviation','+6.6°','crit'],['Backup','Staged','amber']],
    primary:{ label:'Fail over to backup CRAC', kind:'failover' },
    secondary:{ label:'Assign to on-call', kind:'toast', msg:'Assigned to on-call technician · Dan R.' } },
  { id:'peak', sev:'amber', icon:'trending-up', time:'14:18', src:'Forecast',
    title:'Afternoon peak nearing tariff cap',
    say:'Demand is 86.4 kW and rising. I project a 94 kW peak at 2:30pm — six below your demand-charge limit, but close. Pre-cooling Level 4 West by 1.5° now flattens it and avoids a charge worth about $48 today.',
    viz:'demand',
    stats:[['Now','86.4 kW',''],['Projected','94 kW','amber'],['Limit','100 kW',''],['At risk','$48','amber']],
    primary:{ label:'Apply pre-cool to L4 West', kind:'precool' },
    secondary:{ label:'Show the math', kind:'ask', q:'Why is demand high right now?' } },
  { id:'lights', sev:'green', icon:'moon', time:'06:00', src:'Insight',
    title:'Overnight optimisation saved 38 kWh',
    say:'While the building slept, the lighting schedule held Levels 1–4 at 70% and kept demand under 45 kW all night. That\u2019s 38 kWh — about $11 — with no comfort complaints. Worth rolling the same profile to Level 5 commons.',
    viz:'night',
    stats:[['Saved','38 kWh','green'],['Value','$11','green'],['Peak','44 kW',''],['Complaints','0','green']],
    primary:{ label:'Roll out to Level 5', kind:'toast', msg:'Night profile scheduled for Level 5 commons' },
    secondary:{ label:'Dismiss', kind:'dismiss' } },
  { id:'ahu', sev:'muted', icon:'power-off', time:'13:40', src:'Device health',
    title:'L2 West AHU went offline',
    say:'The Level 2 West air handler stopped reporting at 1:40pm — no data for 38 minutes. The zone is unoccupied this afternoon so comfort isn\u2019t at risk yet, but the Modbus gateway may need a reset.',
    viz:'flat',
    stats:[['Offline','38 min','muted'],['Zone','Unoccupied',''],['Gateway','GW-02','']],
    primary:{ label:'Restart gateway GW-02', kind:'toast', msg:'Gateway GW-02 restarting — AHU back in ~40s' },
    secondary:{ label:'Mute for today', kind:'dismiss' } }
];

RX.sevMap = {
  crit:{c:'crit',l:'Critical'}, amber:{c:'amber',l:'Needs action'},
  green:{c:'green',l:'Good news'}, muted:{c:'muted',l:'Watching'}
};

// quick-ask chips
RX.chips = [
  { label:'Why is demand high?', q:'Why is demand high right now?' },
  { label:'Worst zones', q:'Show me the worst zones for comfort' },
  { label:'Watch the chillers for an hour', q:'Build me a live view to watch the chillers for the next hour' },
  { label:'This week vs last', q:'Compare this week to last week' },
  { label:'Draft board report', q:"Draft this week's energy report for the board" }
];

// omni-search index (hidden nav)
RX.nav = {
  views: [
    { id:'overview', label:'Overview', icon:'gauge', meta:'Live building' },
    { id:'energy', label:'Energy & demand', icon:'zap', meta:'Tariff · peaks' },
    { id:'building', label:'Building & zones', icon:'building-2', meta:'8 floors' },
    { id:'rules', label:'Rules', icon:'git-branch', meta:'26 active' },
    { id:'data', label:'Data sources', icon:'database', meta:'48 connectors' },
    { id:'reports', label:'Reports', icon:'file-bar-chart', meta:'Scheduled' },
    { id:'settings', label:'Settings', icon:'settings', meta:'Site & team' }
  ]
};
