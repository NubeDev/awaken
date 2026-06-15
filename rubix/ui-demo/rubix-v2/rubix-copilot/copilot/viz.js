// ============ Rubix Copilot · VIZ PRIMITIVES ============
window.RX = window.RX || {};
RX.v = (function(){
  const C = {
    crit:'hsl(var(--crit))', amber:'hsl(var(--amber))', green:'hsl(var(--green))',
    cool:'hsl(var(--cool))', r1:'hsl(var(--r1))', r2:'hsl(var(--r2))',
    muted:'hsl(var(--muted))', grid:'hsl(230 10% 18%)', axis:'hsl(220 8% 56%)'
  };
  const col = k => C[k] || k;

  function line(pts, opts){
    opts = opts || {};
    const W = opts.w||620, H = opts.h||140, P = opts.p||14;
    const all = pts.flat ? pts.flat() : pts;
    const mn = opts.min!=null?opts.min:Math.min(...all.filter(v=>v!=null)) ;
    const mx = opts.max!=null?opts.max:Math.max(...all.filter(v=>v!=null));
    const X = (i,n)=>P+(i/(n-1))*(W-P*2);
    const Y = v=>H-P-((v-mn)/((mx-mn)||1))*(H-P*2);
    let s = `<svg viewBox="0 0 ${W} ${H}" preserveAspectRatio="none" class="w-full" style="height:${H}px">`;
    for(let g=0;g<=3;g++){const y=P+g*((H-P*2)/3);s+=`<line x1="${P}" y1="${y}" x2="${W-P}" y2="${y}" stroke="${C.grid}" stroke-width="1"/>`;}
    if(opts.setpoint!=null){const y=Y(opts.setpoint);s+=`<line x1="${P}" y1="${y}" x2="${W-P}" y2="${y}" stroke="${C.axis}" stroke-width="1" stroke-dasharray="5 4"/><text x="${W-P-3}" y="${y-5}" fill="${C.axis}" font-size="10" font-family="Geist Mono" text-anchor="end">${opts.setpointLabel||'setpoint'}</text>`;}
    if(opts.limit!=null){const y=Y(opts.limit);s+=`<line x1="${P}" y1="${y}" x2="${W-P}" y2="${y}" stroke="${C.crit}" stroke-width="1.2" stroke-dasharray="6 5" opacity=".7"/><text x="${W-P-3}" y="${y-5}" fill="${C.crit}" font-size="10" font-family="Geist Mono" text-anchor="end" opacity=".85">${opts.limitLabel||'limit'}</text>`;}
    // support multiple series: pts = [{data,color,dash,fill,draw}]
    const series = Array.isArray(pts[0]) || typeof pts[0]==='object' && pts[0].data ? pts : [{data:pts, color:opts.color||'r1', fill:opts.fill}];
    series.forEach(se=>{
      const data = se.data||se; const n=data.length;
      let d='';
      data.forEach((v,i)=>{ if(v==null) return; d+=(d?'L':'M')+X(i,n).toFixed(1)+' '+Y(v).toFixed(1)+' '; });
      const co = col(se.color||opts.color||'r1');
      if(se.fill!==false && (se.fill||opts.fill)) s+=`<path d="${d} L${X(n-1,n)} ${H-P} L${X(0,n)} ${H-P} Z" fill="${co}" opacity=".12"/>`;
      const len = se.draw? ' style="stroke-dasharray:1400;stroke-dashoffset:1400;animation:draw 1.1s ease forwards"' : '';
      s+=`<path d="${d}" fill="none" stroke="${co}" stroke-width="${se.width||2.2}"${se.dash?` stroke-dasharray="${se.dash}"`:''} stroke-linecap="round"${len}/>`;
      if(se.dot!==false){ const li=n-1; s+=`<circle cx="${X(li,n)}" cy="${Y(data[li])}" r="3.5" fill="${co}"/>`; }
    });
    return s+'</svg>';
  }

  function spark(data, color, w, h){
    w=w||120;h=h||34;const P=3;
    const mn=Math.min(...data),mx=Math.max(...data);
    const X=i=>(i/(data.length-1))*w,Y=v=>h-P-((v-mn)/((mx-mn)||1))*(h-P*2);
    const d=data.map((v,i)=>(i?'L':'M')+X(i).toFixed(1)+' '+Y(v).toFixed(1)).join(' ');
    return `<svg viewBox="0 0 ${w} ${h}" preserveAspectRatio="none" style="width:100%;height:${h}px"><path d="${d}" fill="none" stroke="${col(color)}" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/></svg>`;
  }

  function bars(rows){
    const mx=Math.max(...rows.map(r=>r.kw));
    return '<div class="space-y-2.5">'+rows.map(r=>`
      <div class="flex items-center gap-3">
        <div class="w-[92px] text-[13px] text-fg/80 shrink-0">${r.label}</div>
        <div class="flex-1 h-6 rounded-md bg-bg/50 overflow-hidden"><div class="h-full rounded-md" style="width:${(r.kw/mx*100).toFixed(0)}%;background:${col(r.color)};transition:width .7s cubic-bezier(.2,.7,.2,1)"></div></div>
        <div class="w-[92px] text-right mono text-[13px]"><b>${r.kw}</b><span class="text-muted"> kW·${r.pct}%</span></div>
      </div>`).join('')+'</div>';
  }

  function gauge(val, max, color){
    const pct=Math.min(val/max,1), ang=pct*180, r=78, cx=100, cy=96;
    const x=cx+r*Math.cos(Math.PI-ang*Math.PI/180), y=cy-r*Math.sin(ang*Math.PI/180);
    const large=ang>180?1:0;
    return `<svg viewBox="0 0 200 110" style="width:180px;height:99px">
      <path d="M22 96 A78 78 0 0 1 178 96" fill="none" stroke="hsl(230 10% 18%)" stroke-width="12" stroke-linecap="round"/>
      <path d="M22 96 A78 78 0 ${large} 1 ${x.toFixed(1)} ${y.toFixed(1)}" fill="none" stroke="${col(color)}" stroke-width="12" stroke-linecap="round"/>
    </svg>`;
  }

  function donut(segments, size){
    size=size||104;const r=15.9;let off=0;
    let s=`<svg viewBox="0 0 42 42" style="width:${size}px;height:${size}px" class="-rotate-90"><circle cx="21" cy="21" r="${r}" fill="none" stroke="hsl(230 10% 18%)" stroke-width="5"/>`;
    segments.forEach(seg=>{const dash=seg.pct;s+=`<circle cx="21" cy="21" r="${r}" fill="none" stroke="${col(seg.color)}" stroke-width="5" stroke-dasharray="${dash} ${100-dash}" stroke-dashoffset="${-off}" stroke-linecap="round"/>`;off+=dash;});
    return s+'</svg>';
  }

  // temp -> heat color (interpolated)
  function heat(t){
    if(t==null) return 'hsl(38 10% 30%)';
    const st=[[16,196,62,52],[20,150,58,45],[23,150,55,46],[24.5,40,80,52],[26,18,82,52],[27,6,70,50]];
    if(t<=st[0][0]){const s=st[0];return `hsl(${s[1]} ${s[2]}% ${s[3]}%)`;}
    const last=st[st.length-1]; if(t>=last[0]) return `hsl(${last[1]} ${last[2]}% ${last[3]}%)`;
    for(let i=0;i<st.length-1;i++){if(t>=st[i][0]&&t<=st[i+1][0]){const f=(t-st[i][0])/(st[i+1][0]-st[i][0]);
      return `hsl(${(st[i][1]+(st[i+1][1]-st[i][1])*f).toFixed(0)} ${(st[i][2]+(st[i+1][2]-st[i][2])*f).toFixed(0)}% ${(st[i][3]+(st[i+1][3]-st[i][3])*f).toFixed(0)}%)`;}}
    return `hsl(${last[1]} ${last[2]}% ${last[3]}%)`;
  }

  return { col, line, spark, bars, gauge, donut, heat, C };
})();
