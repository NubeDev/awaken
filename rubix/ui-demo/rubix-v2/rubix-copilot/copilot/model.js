// ============ Rubix Copilot · MODEL (portfolio / dashboards / menu) ============
window.RX = window.RX || {};

// ---- portfolio of sites ----
RX.sites = [
  { id:'syd', name:'Sydney HQ', loc:'Sydney · NSW', kind:'Office · 8 floors', energy:'1,284', demand:'86.4', alerts:2, status:'online', grad:'258 84% 64%,174 70% 50%' },
  { id:'mel', name:'Melbourne Tower', loc:'Melbourne · VIC', kind:'Office · 14 floors', energy:'2,910', demand:'142', alerts:0, status:'online', grad:'200 86% 58%,258 84% 68%' },
  { id:'bne', name:'Brisbane DC', loc:'Brisbane · QLD', kind:'Data centre', energy:'7,420', demand:'310', alerts:3, status:'online', grad:'357 84% 60%,32 92% 56%' },
  { id:'per', name:'Perth Plaza', loc:'Perth · WA', kind:'Retail · 6 floors', energy:'940', demand:'58', alerts:0, status:'online', grad:'32 92% 56%,150 64% 48%' },
  { id:'akl', name:'Auckland Hub', loc:'Auckland · NZ', kind:'Office · 5 floors', energy:'612', demand:'41', alerts:1, status:'partial', grad:'150 64% 48%,200 86% 58%' },
  { id:'adl', name:'Adelaide Works', loc:'Adelaide · SA', kind:'Industrial', energy:'3,180', demand:'196', alerts:0, status:'online', grad:'174 70% 50%,258 84% 66%' }
];

// ---- saved dashboards for the open building ----
RX.dashboards = [
  { id:'energy', name:'Energy Overview', icon:'zap', desc:'Demand, cost & carbon at a glance', widgets:[
      { type:'stats', w:12, items:[['Energy today','1,284 kWh',''],['Demand now','86.4 kW','amber'],['Cost today','$342','amber'],['Carbon','512 kg','green']] },
      { type:'chart', w:8, variant:'demand', title:'Demand · today', sub:'kW · projected to 2:30pm peak' },
      { type:'bars', w:4, title:'Live load breakdown', rows:null /*drivers*/ } ] },
  { id:'hvac', name:'HVAC & Comfort', icon:'thermometer', desc:'Every zone, live', widgets:[
      { type:'zones', w:8, title:'Zones · by deviation', rows:null /*zones*/ },
      { type:'stats', w:4, items:[['In band','7 / 8','green'],['Worst','+6.6°','crit'],['Avg RH','47%',''],['Setpoint','22.0°','']] } ] },
  { id:'tariff', name:'Tariff & Demand', icon:'gauge', desc:'Peak management', widgets:[
      { type:'chart', w:12, variant:'demand', title:'Demand vs tariff cap', sub:'peak window 14:00–20:00 · limit 100 kW' },
      { type:'stats', w:12, items:[['Projected peak','94 kW','amber'],['Limit','100 kW',''],['Headroom','6 kW','amber'],['At risk','$48','amber']] } ] },
  { id:'solar', name:'Solar & Battery', icon:'sun', desc:'Generation & storage', widgets:[
      { type:'chart', w:8, variant:'solar', title:'Solar generation · today', sub:'kW' },
      { type:'stats', w:4, items:[['Now','12.1 kW','amber'],['Today','78 kWh','green'],['Battery','68%',''],['Self-use','41%','green']] } ] },
  { id:'carbon', name:'Carbon', icon:'leaf', desc:'Emissions & intensity', widgets:[
      { type:'chart', w:8, variant:'night', title:'Grid intensity · today', sub:'softer overnight' },
      { type:'stats', w:4, items:[['Today','512 kg','green'],['Intensity','0.39','',],['vs avg','−5%','green'],['YTD','83 t','']] } ] }
];

// ---- the Home hub menu (the launcher you land on after opening a site) ----
RX.menu = [
  { id:'overview',   label:'Overview',          icon:'layout-grid',      sub:'Your pinned board' },
  { id:'copilot',    label:'Ask Rubix',        icon:'sparkles',         sub:'2 things need you', accent:true },
  { id:'insights',   label:'Insight Center',    icon:'lightbulb',        sub:'6 new insights' },
  { id:'dashboards', label:'Dashboards',       icon:'layout-dashboard', sub:'5 saved' },
  { id:'building',   label:'Building & Zones',  icon:'building-2',       sub:'8 floors · 1 alert' },
  { id:'rules',      label:'Rules',            icon:'git-branch',        sub:'26 active' },
  { id:'data',       label:'Data Sources',      icon:'database',         sub:'48 connectors' },
  { id:'reports',    label:'Reports',          icon:'file-bar-chart',    sub:'4 scheduled' },
  { id:'devices',    label:'Devices',          icon:'cpu',               sub:'312 points' },
  { id:'settings',   label:'Settings',         icon:'settings',          sub:'Site & team' }
];

// ---- insights feed (Insight Center) ----
RX.insights = [
  { id:'i6', type:'Forecast',    color:'amber', icon:'trending-up',    time:'14:18', title:'94 kW peak projected at 2:30pm', text:'Six below your demand-charge limit. Pre-cooling Level 4 West now avoids it — about $48.', viz:'demand' },
  { id:'i4', type:'Risk',        color:'crit',  icon:'alert-triangle', time:'14:12', title:'CRAC-2 has faulted 3× this month', text:'The short-cycling pattern points to a refrigerant charge issue — worth booking a service before it fails for good.', viz:'temp' },
  { id:'i2', type:'Anomaly',     color:'amber', icon:'activity',       time:'13:55', title:'Level 4 West is warming faster than usual', text:'Rate of rise is 40% above its 30-day pattern for this hour — likely solar gain on the west face.', viz:'temp' },
  { id:'i3', type:'Trend',       color:'green', icon:'trending-down',  time:'Today', title:'Energy is tracking 6% below last week', text:'Almost entirely overnight HVAC. On pace for the best week this quarter.', viz:'compare' },
  { id:'i5', type:'Opportunity', color:'cool',  icon:'sun',            time:'11:00', title:'Shift flexible load into the solar window', text:'From 11am–2pm you generate 30+ kW. Moving EV charging here lifts self-consumption to ~55%.', viz:'solar' },
  { id:'i1', type:'Saving',      color:'green', icon:'moon',           time:'06:00', title:'Overnight optimisation saved 38 kWh', text:'Lighting and a softer HVAC setback held demand under 45 kW all night — about $11, zero complaints.', viz:'night' }
];
