// ============ Rubix Copilot · WIDGETS + ANSWERS ============
window.RX = window.RX || {};
(function(){
  const v = RX.v, S = RX.series;

  // ---- chart variants ----
  function chartFor(variant){
    if(variant==='demand') return v.line(
      [ {data:S.demand, color:'amber', fill:true} ],
      {h:150, max:108, limit:100, limitLabel:'PEAK LIMIT 100 kW'});
    if(variant==='temp') return v.line(S.serverTemp,{h:150,min:17.5,max:25.5,color:'crit',fill:true,setpoint:18,setpointLabel:'setpoint 18°'});
    if(variant==='night') return v.line(S.night,{h:140,min:30,max:60,color:'green',fill:true});
    if(variant==='solar') return v.line(S.solar,{h:140,min:0,max:40,color:'amber',fill:true});
    if(variant==='compare') return v.line(
      [ {data:S.weekLast,color:'muted',dash:'5 4',dot:false}, {data:S.weekThis,color:'r1',fill:true} ],
      {h:150,min:120,max:200});
    if(variant==='consequence') return v.line(
      [ {data:S.demandForecast,color:'amber',dash:'5 4',dot:false}, {data:S.precooled,color:'green',draw:true} ],
      {h:150,min:74,max:102,limit:100,limitLabel:'cap 100 kW'});
    if(variant==='recover') return v.line(
      [ {data:S.serverRecover,color:'green',draw:true} ],
      {h:150,min:17.5,max:25.5,color:'green',setpoint:18,setpointLabel:'setpoint 18°'});
    return v.line(S.demand,{h:140,color:'r1',fill:true});
  }

  // ---- widget renderer ----
  RX.widget = function(w){
    if(typeof w==='string') return w;
    if(w.type==='chart') return card(w.title, w.sub, `<div class="px-1 pt-1">${chartFor(w.variant)}</div>`, w.legend);
    if(w.type==='bars')  return card(w.title, w.sub, v.bars(w.rows));
    if(w.type==='stats') return statRow(w.items);
    if(w.type==='zones') return zoneTable(w.rows, w.title);
    if(w.type==='report') return reportWidget();
    if(w.type==='dash')  return dashWidget(w);
    return '';
  };
  function card(title, sub, body, legend){
    return `<div class="rounded-2xl border border-border bg-bg/40 p-4">
      ${title?`<div class="flex items-center justify-between mb-2"><div><div class="text-[13px] font-semibold">${title}</div>${sub?`<div class="text-[11.5px] text-muted">${sub}</div>`:''}</div>${legend?`<div class="flex gap-3 text-[11px] text-muted">${legend.map(l=>`<span class="flex items-center gap-1.5"><i class="inline-block w-3 h-[3px] rounded" style="background:${v.col(l.c)}"></i>${l.t}</span>`).join('')}</div>`:''}</div>`:''}
      ${body}</div>`;
  }
  function statRow(items){
    return `<div class="grid grid-cols-${items.length} gap-3">${items.map(s=>`<div class="rounded-xl border border-border bg-bg/40 p-3"><div class="text-[11px] text-muted">${s[0]}</div><div class="mono text-[18px] font-semibold mt-0.5" style="${s[2]?`color:${v.col(s[2])}`:''}">${s[1]}</div></div>`).join('')}</div>`;
  }
  function zoneTable(rows, title){
    return `<div class="rounded-2xl border border-border bg-bg/40 overflow-hidden">
      ${title?`<div class="px-4 pt-3 pb-2 text-[13px] font-semibold">${title}</div>`:''}
      <div class="divide-y divide-border">${rows.map((r,i)=>`
        <div class="flex items-center gap-3 px-4 py-2.5">
          <span class="mono text-[12px] text-muted w-4">${i+1}</span>
          <span class="size-7 rounded-md shrink-0" style="background:${v.heat(r.temp)}"></span>
          <div class="flex-1 min-w-0"><div class="text-[13.5px] font-medium truncate">${r.name}</div><div class="text-[11px] text-muted mono">SP ${r.sp.toFixed(1)}° · ${r.note||r.occ}</div></div>
          <span class="mono text-[15px] ${r.sev==='crit'?'text-crit':''}">${r.temp!=null?r.temp.toFixed(1)+'°':'—'}</span>
          <span class="mono text-[12px] w-12 text-right" style="color:${v.col(r.sev==='muted'?'muted':r.sev)}">${r.temp!=null?((r.temp-r.sp>=0?'+':'')+(r.temp-r.sp).toFixed(1)):'—'}</span>
        </div>`).join('')}</div></div>`;
  }
  function reportWidget(){
    return `<div class="rounded-2xl border border-border bg-bg/40 p-5">
      <div class="flex items-center justify-between"><div class="text-[11px] uppercase tracking-wider text-muted">Weekly Energy Report · draft</div><div class="text-[11px] mono text-muted">Wk 24 · Sydney HQ</div></div>
      <div class="serif text-[22px] font-semibold mt-1.5">A calm, efficient week</div>
      <div class="grid grid-cols-3 gap-3 mt-4">
        <div class="rounded-lg bg-panel3 p-3"><div class="text-[11px] text-muted">Energy</div><div class="mono text-[18px] font-semibold">8.9<span class="text-[11px] text-muted"> MWh</span></div><div class="text-[11px] text-green">▼ 4.2%</div></div>
        <div class="rounded-lg bg-panel3 p-3"><div class="text-[11px] text-muted">Cost</div><div class="mono text-[18px] font-semibold">$2.1k</div><div class="text-[11px] text-green">under budget</div></div>
        <div class="rounded-lg bg-panel3 p-3"><div class="text-[11px] text-muted">Incidents</div><div class="mono text-[18px] font-semibold">1</div><div class="text-[11px] text-muted">CRAC-2 fault</div></div>
      </div>
      <div class="mt-3 text-[12.5px] text-fg/70 leading-relaxed">Sections: Executive summary · Demand &amp; tariff · HVAC performance · Carbon · Open actions</div></div>`;
  }
  // composed live dashboard (the "build me a view" wow)
  function dashWidget(w){
    return `<div class="rounded-2xl border border-r1/25 bg-bg/40 p-4" style="box-shadow:0 0 0 1px hsl(var(--r1)/.06)">
      <div class="flex items-center justify-between mb-3">
        <div class="flex items-center gap-2"><i data-lucide="layout-dashboard" class="size-4 text-r1"></i><span class="text-[13px] font-semibold">${w.title}</span></div>
        <span class="text-[10.5px] mono text-r1 border border-r1/30 rounded-full px-2 py-0.5">LIVE · pinned 60 min</span>
      </div>
      <div class="grid grid-cols-2 gap-3">${w.tiles.map(t=>dashTile(t)).join('')}</div></div>`;
  }
  function dashTile(t){
    if(t.gauge) return `<div class="rounded-xl bg-panel3 p-3 flex flex-col items-center justify-center"><div class="text-[11px] text-muted self-start">${t.label}</div>${v.gauge(t.gauge.val,t.gauge.max,t.gauge.color)}<div class="mono text-[15px] font-semibold -mt-3">${t.value}</div></div>`;
    return `<div class="rounded-xl bg-panel3 p-3"><div class="flex items-center justify-between"><div class="text-[11px] text-muted">${t.label}</div>${t.badge?`<span class="text-[10px] mono px-1.5 py-0.5 rounded" style="background:${v.col(t.badge.c)}/.15;color:${v.col(t.badge.c)}">${t.badge.t}</span>`:''}</div>
      <div class="mono text-[20px] font-semibold mt-0.5" style="${t.color?`color:${v.col(t.color)}`:''}">${t.value}<span class="text-[11px] text-muted ml-0.5">${t.unit||''}</span></div>
      ${t.spark?`<div class="mt-1.5">${v.spark(t.spark.data,t.spark.color)}</div>`:''}</div>`;
  }

  // ---- intent routing ----
  function route(q){
    q=q.toLowerCase();
    if(/(watch|build|view|monitor|keep an eye|track).*(chiller|hour|l5|server|zone|hvac)|chiller/.test(q)) return 'compose';
    if(/(compare|vs|versus|last week|this week|week)/.test(q)) return 'compare';
    if(/(report|board|draft|summary|weekly)/.test(q)) return 'report';
    if(/(save|saving|overnight|night|lighting)/.test(q)) return 'savings';
    if(/(server|crac|rack|l5)/.test(q)) return 'server';
    if(/(solar|pv|battery)/.test(q)) return 'solar';
    if(/(zone|worst|comfort|hot|hottest|temperature)/.test(q)) return 'zones';
    if(/(demand|peak|high|load|why|tariff)/.test(q)) return 'demand';
    return 'fallback';
  }

  // ---- answer composer ----
  RX.answer = function(query){
    const intent = route(query);
    const A=(text,widgets,actions)=>({text,widgets:widgets||[],actions:actions||[],intent});

    if(intent==='demand') return A(
      'Demand is 86.4 kW. HVAC is the driver — 54% of it, mostly the Level 4 and 5 chillers fighting the afternoon sun. Here\u2019s the live split, and the projected 94 kW peak at 2:30pm.',
      [{type:'bars',title:'What\u2019s drawing power now',rows:RX.drivers},
       {type:'chart',variant:'demand',title:'Demand · today',sub:'kW · projected to peak'}],
      [{label:'Apply pre-cool to L4 West',kind:'precool',primary:true},{label:'Open Energy',kind:'navigate',to:'energy'}]);

    if(intent==='zones'){
      const ranked=[...RX.zones].filter(z=>z.temp!=null||z.note).sort((a,b)=>Math.abs((b.temp||b.sp)-b.sp)-Math.abs((a.temp||a.sp)-a.sp)).slice(0,5);
      return A('Five zones are worth a look. The server room is the only real problem; L4 West is warm because it\u2019s pre-empting the peak. Everything else is inside tolerance.',
      [{type:'zones',title:'Zones by deviation from setpoint',rows:ranked}],
      [{label:'Open Level 5 · Server',kind:'ask',q:'Why is the server room hot?',primary:true},{label:'View building',kind:'navigate',to:'building'}]);
    }

    if(intent==='compose') return A(
      'Done — I\u2019ve assembled a live view of both chillers and the server room and pinned it for the next hour. I\u2019ll ping you if any of these cross a threshold.',
      [{type:'dash',title:'Chillers & server · live watch',tiles:[
        {label:'L4 chiller load',gauge:{val:14.8,max:25,color:'amber'},value:'14.8 kW'},
        {label:'L5 chiller load',gauge:{val:21.4,max:25,color:'crit'},value:'21.4 kW'},
        {label:'Server inlet',value:'24.6',unit:'°C',color:'crit',badge:{t:'+6.6',c:'crit'},spark:{data:RX.series.serverTemp,color:'crit'}},
        {label:'Supply air',value:'19.2',unit:'°C',spark:{data:[16,16.4,17,17.6,18.2,18.7,19.2],color:'cool'}}
      ]}],
      [{label:'Pin to Overview',kind:'pin',board:'overview',pin:{title:'Chillers & server \u00b7 live watch',from:'Rubix watch',widgets:[{type:'dash',title:'Chillers & server \u00b7 live watch',tiles:[{label:'L4 chiller load',gauge:{val:14.8,max:25,color:'amber'},value:'14.8 kW'},{label:'L5 chiller load',gauge:{val:21.4,max:25,color:'crit'},value:'21.4 kW'},{label:'Server inlet',value:'24.6',unit:'\u00b0C',color:'crit',badge:{t:'+6.6',c:'crit'},spark:{data:RX.series.serverTemp,color:'crit'}},{label:'Supply air',value:'19.2',unit:'\u00b0C',spark:{data:[16,16.4,17,17.6,18.2,18.7,19.2],color:'cool'}}]}]},primary:true},{label:'Pin to Insights',kind:'pin',board:'insights',pin:{title:'Chillers & server \u00b7 live watch',from:'Rubix watch',widgets:[{type:'dash',title:'Chillers & server \u00b7 live watch',tiles:[{label:'L4 chiller load',gauge:{val:14.8,max:25,color:'amber'},value:'14.8 kW'},{label:'L5 chiller load',gauge:{val:21.4,max:25,color:'crit'},value:'21.4 kW'},{label:'Server inlet',value:'24.6',unit:'\u00b0C',color:'crit',badge:{t:'+6.6',c:'crit'},spark:{data:RX.series.serverTemp,color:'crit'}},{label:'Supply air',value:'19.2',unit:'\u00b0C',spark:{data:[16,16.4,17,17.6,18.2,18.7,19.2],color:'cool'}}]}]}},{label:'Stop watching',kind:'dismiss'}]);

    if(intent==='compare') return A(
      'This week is tracking 6% below last week — 1,074 kWh/day on average versus 1,143. The gap is almost entirely overnight HVAC setback. Weekend draw is down most.',
      [{type:'chart',variant:'compare',title:'Daily energy · this week vs last',sub:'kWh/day',legend:[{t:'This week',c:'r1'},{t:'Last week',c:'muted'}]},
       {type:'stats',items:[['This wk avg','1,074',''],['Last wk avg','1,143','muted'],['Change','−6.0%','green'],['Saved','$210','green']]}],
      [{label:'Pin to Overview',kind:'pin',board:'overview',pin:{title:'This week vs last week',from:'Comparison',widgets:[{type:'chart',variant:'compare',title:'This week vs last',legend:[{t:'This week',c:'r1'},{t:'Last week',c:'muted'}]}]},primary:true},{label:'Add to report',kind:'toast',msg:'Comparison added to this week\u2019s report'}]);

    if(intent==='report') return A(
      'Here\u2019s a draft board pack for this week — energy down 4.2%, costs $2.1k under budget, one incident logged. Want it in Reports, or sent straight to the board?',
      [{type:'report'}],
      [{label:'Open in Reports',kind:'navigate',to:'reports',primary:true},{label:'Send to board',kind:'toast',msg:'Report sent to the board · 4 recipients'}]);

    if(intent==='savings') return A(
      'Overnight the lighting schedule and a softer HVAC setback saved 38 kWh — about $11 — with zero complaints. The same profile on Level 5 commons would add roughly $4 a night.',
      [{type:'chart',variant:'night',title:'Overnight demand',sub:'held under 45 kW'},
       {type:'stats',items:[['Saved','38 kWh','green'],['Value','$11','green'],['Complaints','0','green']]}],
      [{label:'Roll out to Level 5',kind:'toast',msg:'Night profile scheduled for Level 5 commons',primary:true},{label:'See the rule',kind:'navigate',to:'rules'}]);

    if(intent==='server') return A(
      RX.moments.find(m=>m.id==='server').say,
      [{type:'chart',variant:'temp',title:'Server room inlet · last hour'}],
      [{label:'Fail over to backup CRAC',kind:'failover',primary:true},{label:'Assign to on-call',kind:'toast',msg:'Assigned to on-call technician · Dan R.'}]);

    if(intent==='solar') return A(
      'Rooftop PV is generating 12.1 kW — about 14% of load — and the battery is at 68%, idle. I\u2019m holding the battery for the 2:30pm peak rather than discharging now.',
      [{type:'chart',variant:'solar',title:'Solar generation · today',sub:'kW'}],
      [{label:'Discharge battery at peak',kind:'toast',msg:'Battery armed to discharge 22 kW at 2:30pm',primary:true},{label:'Open Energy',kind:'navigate',to:'energy'}]);

    return A(`Here\u2019s what I found for “${query}”. Sydney HQ is drawing 86.4 kW across 312 live points; the only thing outside tolerance is the Level 5 server room. Ask me to dig into any zone, rule, or device — or to build you a live view.`,
      [{type:'chart',variant:'demand',title:'Demand · today'}],
      [{label:'Show the worst zones',kind:'ask',q:'Show me the worst zones for comfort',primary:true},{label:'Open Overview',kind:'navigate',to:'overview'}]);
  };
})();
