// ============ Rubix Copilot · APP (router) ============
(function(){
  const $=id=>document.getElementById(id);
  const v=RX.v;
  RX.cur = RX.sites[0];
  let screen='sites', param=null;
  // pinned boards (persisted)
  try{ RX.pins = JSON.parse(localStorage.getItem('rx_pins'))||{overview:[],insights:[]}; }catch(e){ RX.pins={overview:[],insights:[]}; }
  RX.pins.overview=RX.pins.overview||[]; RX.pins.insights=RX.pins.insights||[];
  function savePins(){ try{ localStorage.setItem('rx_pins',JSON.stringify(RX.pins)); }catch(e){} }
  RX.pin=(board,item)=>{ RX.pins[board]=RX.pins[board]||[]; RX.pins[board].unshift(item); savePins(); };
  RX.unpin=(board,i)=>{ RX.pins[board].splice(i,1); savePins(); render(); };
  RX.pinInsight=(id,board)=>{ const n=RX.insights.find(x=>x.id===id); if(!n) return; RX.pin(board,{title:n.title,from:n.type,widgets:[{type:'chart',variant:n.viz,title:n.title}]}); RX.toast('Pinned to '+(board==='overview'?'Overview':'this board')); if(screen==='insights'||screen==='overview') render(); };

  // conversation state (persists across nav)
  let thread=[], queue=RX.moments.map(m=>m.id), acts=[];

  // ---------- ROUTER ----------
  RX.go=(s,p)=>{ screen=s; param=p||null; render(); };
  RX.openSite=(id)=>{ RX.cur=RX.sites.find(x=>x.id===id)||RX.sites[0]; RX.go('home'); };
  function render(){
    const app=$('app');
    if(screen==='sites') app.innerHTML=RX.screens.sites();
    else if(screen==='home') app.innerHTML=RX.screens.home();
    else if(screen==='dashboards') app.innerHTML=RX.screens.dashboards(param);
    else if(screen==='building') app.innerHTML=RX.screens.building();
    else if(screen==='overview') app.innerHTML=RX.screens.overview();
    else if(screen==='insights') app.innerHTML=RX.screens.insights();
    else if(screen==='copilot'){ app.innerHTML=copilotShell(); renderThread(); renderQueue(); }
    else app.innerHTML=RX.screens.page(screen);
    lucide.createIcons();
    const sc=$('thread'); if(sc) sc.scrollTop=sc.scrollHeight;
  }

  // ---------- COPILOT SCREEN ----------
  function copilotShell(){
    return `<div class="h-full flex flex-col">
      ${RX.screens.topbar(['Ask Rubix'])}
      <div class="flex-1 grid grid-cols-[1fr_372px] gap-5 px-8 pb-2 min-h-0">
        <main id="thread" class="min-h-0 overflow-auto space-y-5 pt-4 pr-2"></main>
        <aside class="min-h-0 flex flex-col pt-4">
          <div class="flex items-center justify-between mb-2.5"><div id="qhead" class="text-[12px] uppercase tracking-[.12em] text-muted font-medium">Rubix lined up · by impact</div><span id="qcount" class="text-[11px] text-muted mono"></span></div>
          <div id="queue" class="space-y-2.5 overflow-auto flex-1 pr-1"></div>
        </aside>
      </div>
      <div class="shrink-0 px-8 pb-6 pt-2">
        <div id="chips" class="flex items-center gap-2 mb-2.5 overflow-x-auto">${RX.chips.map(c=>`<button class="chip rounded-full border border-border bg-panel2 px-3 py-1.5 text-[12.5px] text-muted whitespace-nowrap" onclick="RX.ask(this.dataset.q)" data-q="${c.q.replace(/"/g,'&quot;')}">${c.label}</button>`).join('')}</div>
        <div class="relative">
          <div class="absolute left-4 top-1/2 -translate-y-1/2 size-6 grid place-items-center"><div class="absolute inset-0 rounded-full orb-core opacity-90"></div><div class="absolute inset-[3px] rounded-full bg-bg/40"></div></div>
          <input id="ask" autocomplete="off" class="w-full h-[52px] rounded-2xl border border-border bg-panel2 pl-12 pr-28 text-[15px] outline-none placeholder:text-muted focus:border-r1/50 focus:ring-4 focus:ring-r1/10 transition" placeholder="Ask Rubix, or tell it what to do — “watch the chillers for an hour”…" />
          <div class="absolute right-3 top-1/2 -translate-y-1/2 flex items-center gap-2"><button id="send" class="h-9 px-3.5 rounded-xl bg-fg text-bg text-[13px] font-semibold flex items-center gap-1.5 hover:opacity-90 transition">Ask<i data-lucide="arrow-up" class="size-4"></i></button></div>
        </div>
      </div>
    </div>`;
  }
  function orb(){ return `<div class="relative size-8 shrink-0 grid place-items-center mt-0.5"><div class="absolute inset-0 rounded-full orb-core"></div><div class="absolute inset-[3px] rounded-full bg-bg"></div><i data-lucide="sparkles" class="size-4 relative text-white"></i></div>`; }
  function icon(n){ return `<i data-lucide="${n}" class="size-4"></i>`; }
  function momentTurn(id){ const m=RX.moments.find(x=>x.id===id); const s=RX.sevMap[m.sev];
    return { role:'rubix', sev:m.sev, head:{badge:s.l,time:m.time,src:m.src,icon:m.icon,title:m.title}, text:m.say,
      widgets:[{type:'chart',variant:m.viz,title:'Live'},{type:'stats',items:m.stats}], actions:[{...m.primary,primary:true,_m:id},{...m.secondary,_m:id}] }; }

  function renderThread(){ const el=$('thread'); if(!el) return; acts=[]; el.innerHTML=thread.map(renderTurn).join(''); lucide.createIcons(); el.scrollTop=el.scrollHeight; }
  function renderTurn(t){
    if(t.role==='user') return `<div class="flex justify-end fade"><div class="max-w-[72%] rounded-2xl rounded-br-sm bg-panel3 border border-border px-4 py-2.5 text-[14px] text-fg/90">${t.text}</div></div>`;
    if(t.thinking) return `<div class="flex gap-3 fade">${orb()}<div class="flex items-center gap-1.5 h-7 mt-0.5"><span class="size-2 rounded-full bg-r1 dot1"></span><span class="size-2 rounded-full bg-r1 dot2"></span><span class="size-2 rounded-full bg-r1 dot3"></span><span class="text-[12.5px] text-muted ml-1">Rubix is composing…</span></div></div>`;
    const sc=t.sev?RX.sevMap[t.sev].c:'r1';
    const head=t.head?`<div class="flex items-center gap-2.5 mb-2"><span class="inline-flex items-center gap-1.5 rounded-full px-2.5 py-1 text-[11px] font-semibold" style="background:hsl(var(--${sc})/.14);color:hsl(var(--${sc}))"><span class="size-1.5 rounded-full" style="background:hsl(var(--${sc}))"></span>${t.head.badge}</span><span class="text-[11.5px] text-muted mono">${t.head.time} · ${t.head.src}</span></div><h3 class="text-[20px] font-semibold tracking-tight mb-2 flex items-center gap-2.5"><i data-lucide="${t.head.icon}" class="size-5" style="color:hsl(var(--${sc}))"></i>${t.head.title}</h3>`:'';
    const text=t.text?`<p class="serif text-[17px] leading-relaxed text-fg/88">${t.text}</p>`:'';
    const widgets=(t.widgets||[]).map(RX.widget).join('');
    const actions=(t.actions||[]).filter(a=>a&&a.label).map(a=>{ const i=acts.push(a)-1; const col=a.primary?(t.sev?`background:hsl(var(--${sc}));color:hsl(234 18% 8%)`:'background:hsl(var(--r1));color:hsl(234 18% 8%)'):'';
      const inner=a.primary?`<span class="inline-flex items-center gap-1.5">${icon('zap')}${a.label}</span>`:a.label;
      return `<button onclick="RX.fire(${i})" class="h-10 px-4 rounded-xl text-[13.5px] font-${a.primary?'semibold':'medium'} transition hover:opacity-90 ${a.primary?'':'border border-border hover:bg-panel3'}" style="${col}">${inner}</button>`; }).join('');
    return `<div class="flex gap-3 fade">${orb()}<div class="flex-1 min-w-0 space-y-3">${head}${text}${widgets}${actions?`<div class="flex flex-wrap items-center gap-2 pt-1">${actions}</div>`:''}</div></div>`;
  }
  function renderQueue(){ const el=$('queue'); if(!el) return; const calm=queue.length===0;
    $('qcount').textContent=calm?'all clear':queue.length+' open'; $('qhead').textContent=calm?'Nothing needs you':'Rubix lined up · by impact';
    if(calm){ el.innerHTML=`<div class="rounded-2xl border border-green/25 bg-green/[.05] p-6 text-center"><div class="size-12 rounded-full bg-green/15 grid place-items-center mx-auto"><i data-lucide="check" class="size-6 text-green"></i></div><div class="text-[14px] font-semibold mt-3">${RX.cur.name} is calm</div><div class="text-[12.5px] text-muted mt-1 leading-snug">Every zone in band, demand under cap.</div></div>`; lucide.createIcons(); return; }
    el.innerHTML=queue.map(id=>{ const m=RX.moments.find(x=>x.id===id); const s=RX.sevMap[m.sev];
      return `<button onclick="RX.focus('${id}')" class="qitem w-full text-left rounded-2xl border border-border bg-panel2 hover:bg-panel3 p-4 flex items-start gap-3.5"><div class="size-10 rounded-xl grid place-items-center shrink-0" style="background:hsl(var(--${s.c})/.12)"><i data-lucide="${m.icon}" class="size-5" style="color:hsl(var(--${s.c}))"></i></div><div class="min-w-0 flex-1"><div class="flex items-center gap-2"><span class="size-1.5 rounded-full shrink-0" style="background:hsl(var(--${s.c}))"></span><span class="text-[11px] font-medium" style="color:hsl(var(--${s.c}))">${s.l}</span><span class="text-[11px] text-muted mono ml-auto">${m.time}</span></div><div class="text-[14px] font-semibold mt-1 leading-tight">${m.title}</div><div class="text-[12.5px] text-muted mt-1 leading-snug">${m.say.split('. ')[0]}.</div></div></button>`; }).join('');
    lucide.createIcons();
  }

  RX.focus=(id)=>{ if(screen!=='copilot'){RX.go('copilot');} thread.push(momentTurn(id)); renderThread(); };
  RX.fire=(i)=>{ const a=acts[i]; if(a) dispatch(a); };
  function dispatch(a){
    if(a.kind==='ask'){ RX.ask(a.q); return; }
    if(a.kind==='navigate'){ if(a.to==='energy'){RX.go('dashboards','energy');} else {RX.go(a.to);} return; }
    if(a.kind==='pin'){ const b=a.board||'overview'; RX.pin(b,a.pin); RX.toast('Pinned'); thread.push({role:'rubix',text:'Pinned <b>'+a.pin.title+'</b> to your '+(b==='overview'?'Overview':'Insight Center')+'. It\u2019ll stay live there.',actions:[{label:'Open '+(b==='overview'?'Overview':'Insight Center'),kind:'navigate',to:b,primary:true}]}); renderThread(); return; }
    if(a.kind==='precool'){ thread.push({role:'rubix',text:'Applying a 1.5° pre-cool to Level 4 West now. Watch the projected peak fall back under the cap.',widgets:[{type:'chart',variant:'consequence',title:'Projected demand · before vs after',sub:'pre-cool flattens the 2:30pm peak',legend:[{t:'Was 94 kW',c:'amber'},{t:'Now 88 kW',c:'green'}]},{type:'stats',items:[['New peak','88 kW','green'],['Headroom','12 kW','green'],['Saved','$48','green']]}]}); resolve('peak','Pre-cool applied · peak flattened'); return; }
    if(a.kind==='failover'){ thread.push({role:'rubix',text:'Failing over to the backup CRAC. Cooling is back online — the rack inlet is already dropping toward setpoint.',widgets:[{type:'chart',variant:'recover',title:'Server inlet · recovering'},{type:'stats',items:[['Now','24.6°','amber'],['In 8 min','18.1°','green'],['Status','Recovering','green']]}]}); resolve('server','Backup CRAC online · cooling restored'); return; }
    if(a.kind==='dismiss'){ resolve(a._m,'Dismissed'); return; }
    if(a.kind==='toast'){ RX.toast(a.msg||'Done'); if(a._m) resolve(a._m,a.msg||'Done',true); return; }
    RX.toast('Done');
  }
  function resolve(id,msg){ queue=queue.filter(x=>x!==id); if(msg) RX.toast(msg); renderQueue();
    if(queue.length===0){ thread.push({role:'rubix',text:'That\u2019s everything, Avery. '+RX.cur.name+' is running clean — 86 kW and falling, every zone back in band. I\u2019ll keep watch.',widgets:[{type:'stats',items:[['Demand','82 kW','green'],['Comfort','8/8','green'],['Open','0','green'],['Solar','12.1 kW','amber']]}]}); }
    renderThread();
  }
  RX.ask=(q)=>{ q=(q||'').trim(); if(!q) return; if(screen!=='copilot'){ RX.go('copilot'); }
    thread.push({role:'user',text:q}); const tk={role:'rubix',thinking:true}; thread.push(tk); renderThread();
    setTimeout(()=>{ const i=thread.indexOf(tk); if(i>=0) thread.splice(i,1); const ans=RX.answer(q); thread.push({role:'rubix',text:ans.text,widgets:ans.widgets,actions:ans.actions}); renderThread(); },780);
  };

  // ---------- SEARCH (⌘K) ----------
  const pal=$('pal'),palInput=$('palInput'),palResults=$('palResults'); let selIdx=0,flat=[];
  RX.openPal=()=>{ pal.classList.remove('hidden'); palInput.value=''; renderPal(''); setTimeout(()=>palInput.focus(),30); };
  RX.closePal=()=>pal.classList.add('hidden');
  function renderPal(q){ q=q.toLowerCase().trim(); flat=[]; let html='';
    const sites=RX.sites.filter(s=>!q||s.name.toLowerCase().includes(q)||s.loc.toLowerCase().includes(q));
    const navs=RX.menu.filter(m=>!q||m.label.toLowerCase().includes(q));
    const dash=RX.dashboards.filter(d=>!q||d.name.toLowerCase().includes(q));
    const zs=RX.zones.filter(z=>!q||z.name.toLowerCase().includes(q));
    if(sites.length){ html+=grp('Sites'); sites.forEach(s=>{ html+=row(flat.length,`<i data-lucide="building-2" class="size-4 text-muted"></i>`,s.name,s.loc); flat.push(()=>{RX.closePal();RX.openSite(s.id);}); }); }
    if(navs.length){ html+=grp('In '+RX.cur.name); navs.forEach(m=>{ html+=row(flat.length,`<i data-lucide="${m.icon}" class="size-4 text-muted"></i>`,m.label,m.sub); flat.push(()=>{RX.closePal();RX.go(m.id);}); }); }
    if(dash.length){ html+=grp('Dashboards'); dash.forEach(d=>{ html+=row(flat.length,`<i data-lucide="${d.icon}" class="size-4 text-muted"></i>`,d.name,'dashboard'); flat.push(()=>{RX.closePal();RX.go('dashboards',d.id);}); }); }
    if(zs.length){ html+=grp('Zones'); zs.forEach(z=>{ html+=row(flat.length,`<span class="size-3 rounded" style="background:${v.heat(z.temp)}"></span>`,z.name,z.temp!=null?z.temp.toFixed(1)+'°':(z.note||'')); flat.push(()=>{RX.closePal();RX.ask('Tell me about '+z.name);}); }); }
    if(q&&!flat.length) html=`<div class="px-3 py-6 text-center text-[13px] text-muted">No matches — <button class="text-r1 underline" onclick="RX.closePal();RX.ask('${q.replace(/'/g,'')}')">ask Rubix “${q}”</button></div>`;
    palResults.innerHTML=html; selIdx=0; hi(); lucide.createIcons();
  }
  const grp=t=>`<div class="px-2 py-1.5 mt-1 first:mt-0 text-[11px] uppercase tracking-wider text-muted font-medium">${t}</div>`;
  const row=(i,ic,l,m)=>`<button data-i="${i}" onclick="RX._pal(${i})" class="palrow w-full flex items-center gap-3 rounded-lg px-2.5 py-2.5 text-[13.5px] hover:bg-panel3 transition text-left">${ic}<span class="flex-1">${l}</span><span class="text-[11px] text-muted">${m}</span></button>`;
  function hi(){ palResults.querySelectorAll('.palrow').forEach((r,i)=>r.classList.toggle('selrow',i===selIdx)); }
  RX._pal=i=>flat[i]&&flat[i]();
  palInput.addEventListener('input',e=>renderPal(e.target.value));
  palInput.addEventListener('keydown',e=>{ const rows=palResults.querySelectorAll('.palrow');
    if(e.key==='ArrowDown'){e.preventDefault();selIdx=Math.min(selIdx+1,rows.length-1);hi();rows[selIdx]&&rows[selIdx].scrollIntoView({block:'nearest'});}
    if(e.key==='ArrowUp'){e.preventDefault();selIdx=Math.max(selIdx-1,0);hi();rows[selIdx]&&rows[selIdx].scrollIntoView({block:'nearest'});}
    if(e.key==='Enter'){e.preventDefault();flat[selIdx]&&flat[selIdx]();} });
  pal.addEventListener('click',e=>{if(e.target===pal)RX.closePal();});
  document.addEventListener('keydown',e=>{ if((e.metaKey||e.ctrlKey)&&e.key.toLowerCase()==='k'){e.preventDefault();pal.classList.contains('hidden')?RX.openPal():RX.closePal();} if(e.key==='Escape')RX.closePal(); });

  // ask bar + chips delegation (works after re-render via event delegation)
  document.addEventListener('click',e=>{ if(e.target.closest&&e.target.closest('#send')){ const a=$('ask'); RX.ask(a.value); a.value=''; } });
  document.addEventListener('keydown',e=>{ if(e.target&&e.target.id==='ask'&&e.key==='Enter'){ RX.ask(e.target.value); e.target.value=''; } });

  // toast
  RX.toast=(msg)=>{ const t=$('toast'); $('toastmsg').textContent=msg; t.style.opacity=1; t.style.transform='translate(-50%,0)'; clearTimeout(window._tt); window._tt=setTimeout(()=>{t.style.opacity=0;t.style.transform='translate(-50%,8px)';},2600); };

  // seed conversation
  thread.push({role:'rubix',text:'Two things need you, Avery. The <span class="text-crit">server room</span> is overheating, and the <span class="text-amber">afternoon peak</span> is closing on your tariff cap. Everything else is calm — here\u2019s the one that can\u2019t wait.'});
  thread.push(momentTurn('server'));

  render();
})();
