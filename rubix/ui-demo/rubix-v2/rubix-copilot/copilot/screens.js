// ============ Rubix Copilot · SCREENS ============
window.RX = window.RX || {};
RX.screens = (function(){
  const v = RX.v;
  const sevC = { online:'green', partial:'amber', offline:'crit' };

  function topbar(crumbs){
    const c = RX.cur || RX.sites[0];
    return `<header class="h-14 shrink-0 flex items-center gap-3 px-6 border-b border-border">
      <button onclick="RX.go('home')" class="flex items-center gap-2.5 group">
        <div class="relative size-7 grid place-items-center"><div class="absolute inset-0 rounded-full orb-core blur-[1px]"></div><div class="absolute inset-[3px] rounded-full bg-bg/55"></div></div>
        <span class="font-semibold tracking-tight">Rubix</span>
      </button>
      <div class="h-4 w-px bg-border mx-1"></div>
      <button onclick="RX.go('sites')" class="flex items-center gap-2 text-[13px] hover:text-fg transition"><i data-lucide="building-2" class="size-4 text-muted"></i><span class="font-medium">${c.name}</span><i data-lucide="chevrons-up-down" class="size-3.5 text-muted"></i></button>
      ${crumbs?`<i data-lucide="chevron-right" class="size-3.5 text-muted"></i><div class="flex items-center gap-2 text-[13px] text-muted">${crumbs.map((cr,i)=>`<span class="${i===crumbs.length-1?'text-fg font-medium':''}">${cr}</span>`).join('<i data-lucide=\"chevron-right\" class=\"size-3.5\"></i>')}</div>`:''}
      <div class="ml-auto flex items-center gap-5 text-[12px] mono text-muted">
        <span class="flex items-center gap-1.5"><span class="size-1.5 rounded-full bg-green blink"></span>312 live</span>
        <span>Demand <b class="text-fg font-medium">86.4</b> kW</span>
        <span>Solar <b class="text-amber font-medium">12.1</b> kW</span>
      </div>
      <div class="h-4 w-px bg-border mx-1"></div>
      <button onclick="RX.openPal()" class="flex items-center gap-2 h-8 px-2.5 rounded-lg border border-border text-[12px] text-muted hover:text-fg hover:bg-panel2 transition"><i data-lucide="search" class="size-3.5"></i>Search<kbd>⌘K</kbd></button>
      <div class="size-8 rounded-full bg-panel2 border border-border grid place-items-center text-xs font-semibold">AK</div>
    </header>`;
  }

  // ---------- SITES (portfolio entry) ----------
  function sites(){
    return `<div class="h-full flex flex-col">
      <header class="h-14 shrink-0 flex items-center gap-3 px-6">
        <div class="flex items-center gap-2.5"><div class="relative size-7 grid place-items-center"><div class="absolute inset-0 rounded-full orb-core blur-[1px]"></div><div class="absolute inset-[3px] rounded-full bg-bg/55"></div></div><span class="font-semibold tracking-tight">Rubix</span></div>
        <div class="ml-auto flex items-center gap-2"><button onclick="RX.openPal()" class="flex items-center gap-2 h-8 px-2.5 rounded-lg border border-border text-[12px] text-muted hover:text-fg hover:bg-panel2 transition"><i data-lucide="search" class="size-3.5"></i>Search<kbd>⌘K</kbd></button><div class="size-8 rounded-full bg-panel2 border border-border grid place-items-center text-xs font-semibold">AK</div></div>
      </header>
      <div class="flex-1 overflow-auto px-10 py-8">
        <div class="max-w-[1080px] mx-auto">
          <div class="text-[13px] text-muted">Good afternoon, Avery</div>
          <h1 class="serif text-[34px] font-semibold tracking-tight mt-1">Your portfolio</h1>
          <p class="text-[14px] text-muted mt-1.5">Open a site to manage it — or just <button onclick="RX.openPal()" class="text-r1 hover:underline">search</button> across everything. Rubix is watching all six.</p>
          <div class="grid grid-cols-3 gap-4 mt-7">
            ${RX.sites.map(s=>`
              <button onclick="RX.openSite('${s.id}')" class="qitem text-left rounded-2xl border border-border bg-panel2 hover:bg-panel3 overflow-hidden">
                <div class="h-24 relative" style="background:linear-gradient(135deg,hsl(${s.grad}))">
                  <div class="absolute inset-0" style="background:radial-gradient(120px 80px at 80% 20%,rgba(255,255,255,.18),transparent)"></div>
                  <div class="absolute bottom-2.5 left-3.5 text-white"><div class="text-[15px] font-semibold leading-none">${s.name}</div><div class="text-[11.5px] opacity-85 mt-1">${s.loc}</div></div>
                  ${s.alerts?`<span class="absolute top-2.5 right-2.5 inline-flex items-center gap-1 rounded-full bg-black/35 backdrop-blur px-2 py-0.5 text-[11px] text-white font-medium"><span class="size-1.5 rounded-full bg-crit"></span>${s.alerts}</span>`:''}
                </div>
                <div class="p-3.5">
                  <div class="flex items-center gap-2 text-[12px] text-muted"><i data-lucide="${s.kind.includes('Data')?'server':s.kind.includes('Industrial')?'factory':s.kind.includes('Retail')?'store':'building'}" class="size-3.5"></i>${s.kind}</div>
                  <div class="flex items-center gap-4 mt-3">
                    <div><div class="mono text-[17px] font-semibold">${s.energy}</div><div class="text-[10.5px] text-muted">kWh today</div></div>
                    <div><div class="mono text-[17px] font-semibold">${s.demand}</div><div class="text-[10.5px] text-muted">kW now</div></div>
                    <div class="ml-auto flex items-center gap-1.5 text-[11.5px] ${s.status==='online'?'text-green':'text-amber'}"><span class="size-1.5 rounded-full bg-${s.status==='online'?'green':'amber'}"></span>${s.status==='online'?'Online':'Partial'}</div>
                  </div>
                </div>
              </button>`).join('')}
          </div>
        </div>
      </div>
    </div>`;
  }

  // ---------- HOME HUB (the menu you land on) ----------
  function home(){
    const c = RX.cur;
    return `<div class="h-full flex flex-col">
      ${topbar()}
      <div class="flex-1 overflow-auto px-10 py-8">
        <div class="max-w-[1080px] mx-auto">
          <div class="flex items-start gap-5">
            <div class="size-14 rounded-2xl grid place-items-center text-white shrink-0" style="background:linear-gradient(135deg,hsl(${c.grad}))"><i data-lucide="building-2" class="size-7"></i></div>
            <div class="flex-1">
              <h1 class="serif text-[30px] font-semibold tracking-tight leading-none">${c.name}</h1>
              <div class="text-[13px] text-muted mt-1.5">${c.loc} · ${c.kind}</div>
            </div>
          </div>
          <!-- Rubix attention banner -->
          <button onclick="RX.go('copilot')" class="w-full text-left mt-6 rounded-2xl border border-r1/25 bg-gradient-to-r from-r1/10 to-transparent p-4 flex items-center gap-4 hover:from-r1/15 transition">
            <div class="relative size-10 grid place-items-center shrink-0"><div class="absolute inset-0 rounded-full orb-core"></div><div class="absolute inset-[3px] rounded-full bg-bg"></div><i data-lucide="sparkles" class="size-4 relative text-white"></i></div>
            <div class="flex-1"><div class="serif text-[16px] text-fg/95">Two things need you — the <span class="text-crit">server room</span> is hot and the <span class="text-amber">afternoon peak</span> is near your cap.</div></div>
            <span class="inline-flex items-center gap-1.5 rounded-lg bg-r1/15 text-r1 px-3 py-2 text-[13px] font-semibold shrink-0">Ask Rubix<i data-lucide="arrow-right" class="size-4"></i></span>
          </button>
          <!-- stat strip -->
          <div class="grid grid-cols-4 gap-3 mt-4">
            ${RX.vitals.map(x=>`<div class="rounded-xl border border-border bg-panel2 px-4 py-3"><div class="text-[11.5px] text-muted">${x.label}</div><div class="mono text-[20px] font-semibold mt-0.5">${x.value}<span class="text-[11px] text-muted ml-1">${x.unit}</span></div></div>`).join('')}
          </div>
          <!-- THE MENU -->
          <div class="text-[12px] uppercase tracking-[.12em] text-muted font-medium mt-8 mb-3">Manage ${c.name}</div>
          <div class="grid grid-cols-4 gap-3">
            ${RX.menu.map(m=>`
              <button onclick="RX.go('${m.id}')" class="qitem text-left rounded-2xl border ${m.accent?'border-r1/30 bg-r1/[.06]':'border-border bg-panel2'} hover:bg-panel3 p-4">
                <div class="size-10 rounded-xl grid place-items-center ${m.accent?'':'bg-panel3'}" style="${m.accent?'background:linear-gradient(135deg,hsl(258 84% 64%),hsl(174 70% 50%))':''}"><i data-lucide="${m.icon}" class="size-5 ${m.accent?'text-white':'text-fg'}"></i></div>
                <div class="text-[14.5px] font-semibold mt-3">${m.label}</div>
                <div class="text-[12px] text-muted mt-0.5">${m.sub}</div>
              </button>`).join('')}
          </div>
          <!-- quick dashboards -->
          <div class="text-[12px] uppercase tracking-[.12em] text-muted font-medium mt-8 mb-3">Jump to a dashboard</div>
          <div class="flex gap-2 flex-wrap">
            ${RX.dashboards.map(d=>`<button onclick="RX.go('dashboards','${d.id}')" class="qitem flex items-center gap-2 rounded-xl border border-border bg-panel2 hover:bg-panel3 px-3.5 py-2.5 text-[13px]"><i data-lucide="${d.icon}" class="size-4 text-muted"></i>${d.name}</button>`).join('')}
          </div>
        </div>
      </div>
    </div>`;
  }

  // ---------- DASHBOARDS (with switcher) ----------
  function dashboards(activeId){
    const d = RX.dashboards.find(x=>x.id===activeId) || RX.dashboards[0];
    return `<div class="h-full flex flex-col">
      ${topbar(['Dashboards', d.name])}
      <div class="flex-1 grid grid-cols-[220px_1fr] min-h-0">
        <aside class="border-r border-border p-3 overflow-auto">
          <div class="text-[11px] uppercase tracking-wider text-muted font-medium px-2 py-1.5">Saved dashboards</div>
          ${RX.dashboards.map(x=>`<button onclick="RX.go('dashboards','${x.id}')" class="w-full flex items-center gap-2.5 rounded-lg px-2.5 py-2.5 text-[13.5px] transition ${x.id===d.id?'bg-panel3 text-fg':'text-muted hover:bg-panel2 hover:text-fg'}"><i data-lucide="${x.icon}" class="size-4"></i><span class="flex-1 text-left">${x.name}</span></button>`).join('')}
          <button onclick="RX.toast('New dashboard — drag widgets to build')" class="w-full flex items-center gap-2.5 rounded-lg px-2.5 py-2.5 text-[13px] text-muted hover:bg-panel2 hover:text-fg transition mt-1"><i data-lucide="plus" class="size-4"></i>New dashboard</button>
        </aside>
        <div class="overflow-auto p-6">
          <div class="flex items-center justify-between mb-4"><div><h1 class="text-[20px] font-semibold tracking-tight">${d.name}</h1><div class="text-[13px] text-muted mt-0.5">${d.desc}</div></div>
            <button onclick="RX.toast('Editing ${d.name}')" class="h-9 px-3.5 rounded-lg border border-border text-[13px] font-medium hover:bg-panel2 transition flex items-center gap-2"><i data-lucide="pencil" class="size-4"></i>Edit</button></div>
          <div class="grid grid-cols-12 gap-4">
            ${d.widgets.map(w=>{ const ww={...w}; if(ww.type==='bars'&&!ww.rows) ww.rows=RX.drivers; if(ww.type==='zones'&&!ww.rows) ww.rows=RX.zones; return `<div style="grid-column:span ${w.w||12}">${RX.widget(ww)}</div>`; }).join('')}
          </div>
        </div>
      </div>
    </div>`;
  }

  // ---------- BUILDING & ZONES ----------
  function building(){
    const floors=[...RX.zones];
    function cells(z){ if(z.temp==null) return Array(6).fill(`<span class="flex-1 h-7 rounded" style="background:hsl(38 10% 22%)"></span>`).join(''); 
      return Array.from({length:6},(_,i)=>{const t=z.temp+(Math.sin(i*1.7)*0.5);return `<span class="flex-1 h-7 rounded" style="background:${v.heat(t)}"></span>`;}).join(''); }
    const mx=Math.max(...floors.map(f=>f.load));
    return `<div class="h-full flex flex-col">
      ${topbar(['Building & Zones'])}
      <div class="flex-1 overflow-auto p-6">
        <div class="max-w-[1080px] mx-auto grid grid-cols-[1fr_300px] gap-5">
          <div class="rounded-2xl border border-border bg-panel2 overflow-hidden">
            <div class="px-5 py-3.5 border-b border-border flex items-center justify-between"><div class="font-semibold text-[15px]">Live floor plan</div><div class="text-[12px] text-muted">8 floors · temperature</div></div>
            <div class="divide-y divide-border">
              ${floors.map(z=>{const s=RX.sevMap[z.sev];return `<div class="flex items-center gap-4 px-5 py-3 hover:bg-panel3/50 transition" style="${z.sev==='crit'?'box-shadow:inset 3px 0 0 hsl(var(--crit))':z.sev==='amber'?'box-shadow:inset 3px 0 0 hsl(var(--amber))':''}">
                <div class="w-[132px] shrink-0"><div class="text-[13px] font-semibold">${z.name}</div><div class="text-[11px] text-muted mono">SP ${z.sp.toFixed(1)}°${z.note?' · '+z.note:''}</div></div>
                <div class="flex-1 flex gap-1">${cells(z)}</div>
                <div class="w-[84px] text-right shrink-0"><div class="mono text-[15px] font-semibold ${z.sev==='crit'?'text-crit':''}">${z.temp!=null?z.temp.toFixed(1)+'°':'—'}</div><div class="h-1.5 rounded-full bg-border mt-1 overflow-hidden"><span class="block h-full rounded-full" style="width:${(z.load/mx*100)||0}%;background:hsl(var(--${s.c==='muted'?'muted':s.c}))"></span></div></div>
              </div>`;}).join('')}
            </div>
          </div>
          <div class="space-y-4">
            <div class="rounded-2xl border border-border bg-panel2 p-5">
              <div class="font-semibold text-[15px] mb-3">Comfort</div>
              <div class="flex items-center gap-4"><div class="relative">${v.donut([{pct:75,color:'green'},{pct:12,color:'amber'},{pct:13,color:'crit'}],96)}<div class="absolute inset-0 grid place-content-center text-center"><div class="mono text-[18px] font-semibold">7/8</div><div class="text-[10px] text-muted">in band</div></div></div>
              <div class="space-y-1.5 text-[12.5px] flex-1"><div class="flex items-center gap-2"><span class="size-2 rounded bg-green"></span>Optimal<b class="ml-auto mono">6</b></div><div class="flex items-center gap-2"><span class="size-2 rounded bg-amber"></span>Warm<b class="ml-auto mono">1</b></div><div class="flex items-center gap-2"><span class="size-2 rounded bg-crit"></span>Fault<b class="ml-auto mono">1</b></div></div></div>
            </div>
            ${RX.widget({type:'stats',items:[['Avg temp','22.4°',''],['Avg RH','47%','']]})}
            <button onclick="RX.go('copilot')" class="w-full rounded-2xl border border-r1/25 bg-r1/[.06] p-4 text-left hover:bg-r1/10 transition"><div class="flex items-center gap-2 text-[13px] font-semibold"><i data-lucide="sparkles" class="size-4 text-r1"></i>Rubix</div><div class="text-[12.5px] text-muted mt-1.5">L5 server room needs action — failover ready.</div></button>
          </div>
        </div>
      </div>
    </div>`;
  }

  // ---------- NATIVE PAGES ----------
  function page(id){
    const map={
      rules:{icon:'git-branch',title:'Rules',sub:'26 active automations',rows:['Critical temp → page on-call','Peak shave → dispatch battery at 95 kW','Night setback · 22:00–06:00','CO₂ > 800 ppm → boost fresh air','Tariff peak → pre-cool west floors','AHU offline 10 min → restart gateway']},
      data:{icon:'database',title:'Data Sources',sub:'48 connectors · 312 live points',rows:['BMS · BACnet/IP — 184 points','Meters · Modbus TCP — 64 points','Solar · SunSpec — 12 points','Weather · API — 8 points','Tenant submeters — 36 points','Occupancy · API — 8 points']},
      reports:{icon:'file-bar-chart',title:'Reports',sub:'4 scheduled',rows:['Weekly board pack · Mondays 8am','Monthly energy · 1st of month','NABERS quarterly','Tenant sub-bills · monthly','Carbon disclosure · annual']},
      devices:{icon:'cpu',title:'Devices',sub:'312 points · 298 online',rows:['CRAC-2 · L5 — fault','AHU-2W · L2 — offline','Chiller 1–3 · plant — online','Battery · 68% — idle','Main meter — online','Gateways GW-01…04 — 3 online']},
      settings:{icon:'settings',title:'Settings',sub:'Site & team',rows:['Sydney HQ · 8 floors','Tariff · TOU commercial','6 team members','3 integrations','Alert routing · on-call roster','API keys · 2 active']}
    };
    const p=map[id]||map.settings;
    return `<div class="h-full flex flex-col">
      ${topbar([p.title])}
      <div class="flex-1 overflow-auto p-6">
        <div class="max-w-[760px] mx-auto">
          <div class="flex items-center gap-3"><div class="size-11 rounded-xl bg-panel2 border border-border grid place-items-center"><i data-lucide="${p.icon}" class="size-5 text-muted"></i></div><div><h1 class="text-[22px] font-semibold tracking-tight">${p.title}</h1><div class="text-[13px] text-muted">${p.sub}</div></div>
            <button onclick="RX.toast('Add new ${p.title.toLowerCase()}')" class="ml-auto h-9 px-3.5 rounded-lg border border-border text-[13px] font-medium hover:bg-panel2 transition flex items-center gap-2"><i data-lucide="plus" class="size-4"></i>New</button></div>
          <div class="mt-5 rounded-2xl border border-border bg-panel2 divide-y divide-border overflow-hidden">
            ${p.rows.map(r=>{const fault=/fault|offline/i.test(r);return `<div class="flex items-center gap-3 px-4 py-3.5 hover:bg-panel3/60 transition"><span class="size-1.5 rounded-full bg-${fault?'crit':'green'}"></span><span class="text-[13.5px] flex-1">${r}</span><i data-lucide="chevron-right" class="size-4 text-muted"></i></div>`;}).join('')}
          </div>
          <div class="mt-4 rounded-xl border border-r1/20 bg-r1/5 p-4 text-[12.5px] text-fg/80 flex items-start gap-2.5"><i data-lucide="sparkles" class="size-4 text-r1 mt-0.5 shrink-0"></i>Rubix manages most of ${p.title.toLowerCase()} automatically and surfaces anything that needs a decision in <button onclick="RX.go('copilot')" class="text-r1 hover:underline">Ask Rubix</button>.</div>
        </div>
      </div>
    </div>`;
  }

  // ---------- OVERVIEW (pinnable board) ----------
  function overview(){
    const pins = (RX.pins&&RX.pins.overview)||[];
    return `<div class="h-full flex flex-col">
      ${topbar(['Overview'])}
      <div class="flex-1 overflow-auto p-6"><div class="max-w-[1080px] mx-auto">
        <div class="flex items-end justify-between">
          <div><h1 class="serif text-[26px] font-semibold tracking-tight">Overview</h1><div class="text-[13px] text-muted mt-1">Your board for ${RX.cur.name} — live vitals plus anything you pin from Rubix or Insights.</div></div>
          <button onclick="RX.go('copilot')" class="h-9 px-3.5 rounded-lg border border-r1/30 bg-r1/[.06] text-[13px] font-medium text-r1 hover:bg-r1/10 transition flex items-center gap-2"><i data-lucide="sparkles" class="size-4"></i>Ask Rubix to pin something</button>
        </div>
        <div class="grid grid-cols-4 gap-3 mt-5">${RX.vitals.map(x=>`<div class="rounded-xl border border-border bg-panel2 px-4 py-3"><div class="text-[11.5px] text-muted">${x.label}</div><div class="mono text-[20px] font-semibold mt-0.5">${x.value}<span class="text-[11px] text-muted ml-1">${x.unit}</span></div><div class="text-[11px] mt-0.5 ${x.good?'text-green':'text-amber'}">${x.delta}</div></div>`).join('')}</div>
        <div class="mt-4">${RX.widget({type:'chart',variant:'demand',title:'Demand · today',sub:'kW · always on your Overview'})}</div>
        <div class="flex items-center gap-2 mt-7 mb-3"><div class="text-[12px] uppercase tracking-[.12em] text-muted font-medium">Pinned by you</div><span class="text-[11px] text-muted mono">${pins.length}</span></div>
        ${pins.length? `<div class="grid grid-cols-2 gap-4">${pins.map((p,i)=>pinCard(p,'overview',i)).join('')}</div>`
          : `<div class="rounded-2xl border border-dashed border-border p-8 text-center"><div class="size-11 rounded-xl bg-panel2 grid place-items-center mx-auto"><i data-lucide="pin" class="size-5 text-muted"></i></div><div class="text-[14px] font-semibold mt-3">Nothing pinned yet</div><div class="text-[12.5px] text-muted mt-1">Ask Rubix to “watch the chillers for an hour”, then hit <b>Pin to Overview</b> — or pin an insight from the Insight Center.</div></div>`}
      </div></div></div>`;
  }
  function pinCard(p,board,i){
    return `<div class="rounded-2xl border border-border bg-panel2 p-4">
      <div class="flex items-center gap-2 mb-3"><i data-lucide="pin" class="size-3.5 text-r1"></i><div class="text-[13px] font-semibold flex-1">${p.title}</div>
        <span class="text-[10.5px] text-muted mono">${p.from||'Rubix'}</span>
        <button onclick="RX.unpin('${board}',${i})" class="size-7 grid place-items-center rounded-lg text-muted hover:text-fg hover:bg-panel3 transition"><i data-lucide="x" class="size-4"></i></button></div>
      <div class="space-y-3">${(p.widgets||[]).map(RX.widget).join('')}</div></div>`;
  }

  // ---------- INSIGHT CENTER (feed + pinnable board) ----------
  function insights(){
    const pins=(RX.pins&&RX.pins.insights)||[];
    return `<div class="h-full flex flex-col">
      ${topbar(['Insight Center'])}
      <div class="flex-1 overflow-auto p-6"><div class="max-w-[1080px] mx-auto">
        <div class="flex items-end justify-between">
          <div><h1 class="serif text-[26px] font-semibold tracking-tight">Insight Center</h1><div class="text-[13px] text-muted mt-1">Everything Rubix has noticed at ${RX.cur.name}. Pin the ones that matter — here or to your Overview.</div></div>
          <div class="flex items-center gap-2 text-[12px] text-muted"><span class="flex items-center gap-1.5"><span class="size-1.5 rounded-full bg-r1"></span>${RX.insights.length} insights</span></div>
        </div>
        ${pins.length?`<div class="mt-5"><div class="text-[12px] uppercase tracking-[.12em] text-muted font-medium mb-2.5">Pinned to this board</div><div class="grid grid-cols-2 gap-4">${pins.map((p,i)=>pinCard(p,'insights',i)).join('')}</div></div>`:''}
        <div class="text-[12px] uppercase tracking-[.12em] text-muted font-medium mt-7 mb-3">All insights</div>
        <div class="grid grid-cols-2 gap-4">
          ${RX.insights.map(n=>`
            <div class="rounded-2xl border border-border bg-panel2 p-5 flex flex-col">
              <div class="flex items-center gap-2.5">
                <div class="size-9 rounded-xl grid place-items-center shrink-0" style="background:hsl(var(--${n.color})/.14)"><i data-lucide="${n.icon}" class="size-[18px]" style="color:hsl(var(--${n.color}))"></i></div>
                <span class="text-[11px] font-semibold px-2 py-0.5 rounded-full" style="background:hsl(var(--${n.color})/.14);color:hsl(var(--${n.color}))">${n.type}</span>
                <span class="text-[11px] text-muted mono ml-auto">${n.time}</span>
              </div>
              <div class="text-[15px] font-semibold mt-3 leading-snug">${n.title}</div>
              <p class="text-[13px] text-muted mt-1.5 leading-relaxed flex-1">${n.text}</p>
              <div class="mt-3 rounded-xl border border-border bg-bg/40 p-2">${RX.widget({type:'chart',variant:n.viz})}</div>
              <div class="flex items-center gap-2 mt-3">
                <button onclick="RX.pinInsight('${n.id}','overview')" class="h-9 px-3 rounded-lg text-[12.5px] font-semibold bg-r1 text-bg hover:opacity-90 transition flex items-center gap-1.5"><i data-lucide="pin" class="size-3.5"></i>Pin to Overview</button>
                <button onclick="RX.pinInsight('${n.id}','insights')" class="h-9 px-3 rounded-lg text-[12.5px] font-medium border border-border hover:bg-panel3 transition">Pin here</button>
                <button onclick="RX.ask('Tell me more about: ${n.title.replace(/'/g,'')}')" class="h-9 px-3 rounded-lg text-[12.5px] font-medium text-muted hover:text-fg transition ml-auto">Ask Rubix</button>
              </div>
            </div>`).join('')}
        </div>
      </div></div></div>`;
  }

  return { topbar, sites, home, dashboards, building, page, overview, insights };
})();
