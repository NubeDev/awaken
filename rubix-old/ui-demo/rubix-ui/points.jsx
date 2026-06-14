/* RUBIX — Points & Equipment browser + priority array */

function Points({ activeSite }) {
  const [selEquip, setSelEquip] = React.useState('e2');
  const [selPoint, setSelPoint] = React.useState('p4'); // cooling-valve (fault)
  const [q, setQ] = React.useState('');
  const equip = EQUIPS.find(e => e.id === selEquip);
  const points = POINTS.filter(p => p.equip === selEquip);
  const point = POINTS.find(p => p.id === selPoint) || points[0];

  const filtered = EQUIPS.filter(e => !q || e.name.toLowerCase().includes(q.toLowerCase()) || e.tags.some(t => t.includes(q.toLowerCase())));

  return (
    <div style={{ flex: 1, display: 'flex', minHeight: 0 }}>
      {/* tree */}
      <div style={{ width: 290, flex: 'none', borderRight: '1px solid var(--border)', display: 'flex', flexDirection: 'column', minHeight: 0 }}>
        <div style={{ padding: 12, borderBottom: '1px solid var(--border)' }}>
          <div className="input" style={{ height: 34 }}>
            <Icon name="search" size={15} style={{ color: 'var(--muted-foreground)', marginRight: 8 }} />
            <input value={q} onChange={e => setQ(e.target.value)} placeholder="Filter equipment or #tag" style={{ flex: 1, background: 'none', border: 'none', outline: 'none', fontSize: 13 }} />
          </div>
        </div>
        <div className="scroll" style={{ flex: 1, overflowY: 'auto', padding: 8 }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 7, padding: '6px 8px', fontSize: 12, fontWeight: 600, color: 'var(--muted-foreground)' }}>
            <Icon name="building" size={14} /> {activeSite.name}
          </div>
          {filtered.map(e => {
            const on = e.id === selEquip;
            return (
              <button key={e.id} onClick={() => { setSelEquip(e.id); const fp = POINTS.find(p => p.equip === e.id); if (fp) setSelPoint(fp.id); }} style={{
                display: 'flex', alignItems: 'center', gap: 9, width: '100%', textAlign: 'left', padding: '8px 9px 8px 18px', borderRadius: 8, marginBottom: 1,
                background: on ? 'var(--accent-bg)' : 'transparent', fontSize: 13,
              }} onMouseEnter={ev => { if (!on) ev.currentTarget.style.background = 'var(--subtle)'; }} onMouseLeave={ev => { if (!on) ev.currentTarget.style.background = 'transparent'; }}>
                <Icon name={e.icon} size={16} style={{ color: on ? 'var(--primary)' : 'var(--muted-foreground)' }} />
                <span style={{ flex: 1, fontWeight: on ? 600 : 500, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{e.name}</span>
                {e.alarm && <span style={{ width: 7, height: 7, borderRadius: 99, background: 'var(--sev-fault)' }} />}
              </button>
            );
          })}
        </div>
      </div>

      {/* point list */}
      <div style={{ width: 360, flex: 'none', borderRight: '1px solid var(--border)', display: 'flex', flexDirection: 'column', minHeight: 0 }}>
        <div style={{ padding: '13px 16px', borderBottom: '1px solid var(--border)', display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
          <div>
            <div style={{ fontSize: 13.5, fontWeight: 600 }}>{equip.name}</div>
            <div className="muted mono" style={{ fontSize: 11, marginTop: 2 }}>{activeSite.org}/{activeSite.slug}/{equip.path}</div>
          </div>
          <span className="badge badge-muted">{points.length} pts</span>
        </div>
        <div style={{ display: 'flex', gap: 6, padding: '8px 14px', borderBottom: '1px solid var(--border)', flexWrap: 'wrap' }}>
          {equip.tags.map(t => <span key={t} className="badge badge-outline mono" style={{ fontSize: 10.5 }}>#{t}</span>)}
        </div>
        <div className="scroll" style={{ flex: 1, overflowY: 'auto', padding: 8 }}>
          {points.map(p => {
            const on = p.id === point.id;
            return (
              <button key={p.id} onClick={() => setSelPoint(p.id)} style={{
                display: 'flex', alignItems: 'center', gap: 11, width: '100%', textAlign: 'left', padding: '10px 11px', borderRadius: 9, marginBottom: 2,
                background: on ? 'var(--accent-bg)' : 'transparent', border: '1px solid ' + (on ? 'color-mix(in oklch, var(--primary) 22%, transparent)' : 'transparent'),
              }} onMouseEnter={ev => { if (!on) ev.currentTarget.style.background = 'var(--subtle)'; }} onMouseLeave={ev => { if (!on) ev.currentTarget.style.background = 'transparent'; }}>
                <PointKindDot kind={p.kind} status={p.status} />
                <div style={{ flex: 1, minWidth: 0 }}>
                  <div style={{ fontSize: 12.5, fontWeight: 500, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{p.name}</div>
                  <div className="muted mono" style={{ fontSize: 10.5, marginTop: 1 }}>{p.slug}</div>
                </div>
                <div style={{ textAlign: 'right' }}>
                  <div className="tabular" style={{ fontSize: 13, fontWeight: 650 }}>{p.cur}<span className="muted" style={{ fontSize: 10.5, fontWeight: 400 }}> {p.unit}</span></div>
                  <div className="muted" style={{ fontSize: 10 }}>{p.ts}</div>
                </div>
              </button>
            );
          })}
        </div>
      </div>

      {/* point detail */}
      <div className="scroll" style={{ flex: 1, overflowY: 'auto', minWidth: 0 }}>
        <PointDetail key={point.id} p={point} equip={equip} site={activeSite} />
      </div>
    </div>
  );
}

function PointKindDot({ kind, status }) {
  const map = { sensor: { c: 'var(--chart-2)', i: 'eye' }, cmd: { c: 'var(--chart-1)', i: 'square-pen' }, sp: { c: 'var(--chart-5)', i: 'sliders-horizontal' } };
  const m = map[kind];
  return (
    <span style={{ width: 28, height: 28, borderRadius: 8, flex: 'none', display: 'grid', placeItems: 'center', background: 'var(--subtle)', border: '1px solid var(--border)', color: status === 'fault' ? 'var(--sev-fault)' : m.c, position: 'relative' }}>
      <Icon name={m.i} size={14} />
      {status === 'fault' && <span style={{ position: 'absolute', top: -3, right: -3, width: 9, height: 9, borderRadius: 99, background: 'var(--sev-fault)', boxShadow: '0 0 0 2px var(--background)' }} />}
    </span>
  );
}

function PointDetail({ p, equip, site }) {
  const writable = p.kind === 'cmd' || p.kind === 'sp';
  const hist = series(80, typeof p.cur === 'number' ? p.cur : 50, p.amp || 12, p.seed || 5, { trend: p.status === 'fault' ? 0.15 : 0, period: 20 });
  return (
    <div style={{ padding: 22, maxWidth: 880 }}>
      {/* header */}
      <div style={{ display: 'flex', alignItems: 'flex-start', gap: 14, marginBottom: 18 }}>
        <PointKindDot kind={p.kind} status={p.status} />
        <div style={{ flex: 1, minWidth: 0 }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 9 }}>
            <h2 style={{ fontSize: 18, letterSpacing: '-0.02em' }}>{p.name}</h2>
            <span className="badge badge-muted" style={{ textTransform: 'uppercase', fontSize: 10 }}>{p.kind}</span>
            {p.status === 'fault' && <span className="badge bg-sev-fault"><Icon name="circle-alert" size={12} /> in finding</span>}
          </div>
          <div className="muted mono" style={{ fontSize: 11.5, marginTop: 4 }}>{site.org}/{site.slug}/{equip.path}/{p.slug}/cur</div>
        </div>
        <div style={{ display: 'flex', gap: 7 }}>
          <button className="btn btn-outline btn-sm"><Icon name="pin" size={14} /> Pin</button>
          <button className="btn btn-outline btn-sm btn-icon"><Icon name="history" size={15} /></button>
        </div>
      </div>

      {/* live value + tags */}
      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr 1fr', gap: 12, marginBottom: 16 }}>
        <div className="card" style={{ padding: '14px 16px' }}>
          <div className="eyebrow" style={{ marginBottom: 8 }}>Live value</div>
          <div style={{ display: 'flex', alignItems: 'baseline', gap: 5 }}>
            <span className="tabular" style={{ fontSize: 30, fontWeight: 700, letterSpacing: '-0.04em' }}>{p.cur}</span>
            <span className="muted" style={{ fontSize: 14 }}>{p.unit}</span>
          </div>
          <div className="muted" style={{ fontSize: 11.5, marginTop: 4, display: 'flex', alignItems: 'center', gap: 6 }}><span className="live-dot" /> updated {p.ts} ago</div>
        </div>
        <div className="card" style={{ padding: '14px 16px' }}>
          <div className="eyebrow" style={{ marginBottom: 8 }}>Source · effective</div>
          {writable ? <EffectiveSource p={p} /> : <div style={{ fontSize: 13, color: 'var(--muted-foreground)' }}><Icon name="eye" size={15} style={{ verticalAlign: -2 }} /> Read-only field input</div>}
        </div>
        <div className="card" style={{ padding: '14px 16px' }}>
          <div className="eyebrow" style={{ marginBottom: 8 }}>Tags</div>
          <div style={{ display: 'flex', flexWrap: 'wrap', gap: 5 }}>
            {p.tags.map(t => <span key={t} className="badge badge-outline mono" style={{ fontSize: 10.5 }}>#{t}</span>)}
          </div>
        </div>
      </div>

      {/* history */}
      <div className="card" style={{ padding: 18, marginBottom: 16 }}>
        <SectionHead title="History" sub="Served by local store · Parquet partitions">
          <Segmented size="sm" options={['1h', '24h', '7d', '30d']} value="24h" onChange={() => {}} />
        </SectionHead>
        <AreaChart h={180} series={[{ data: hist, color: p.status === 'fault' ? 'var(--sev-fault)' : 'var(--chart-1)' }]} labels={Array.from({ length: 80 }, (_, i) => `${i}`)} />
      </div>

      {/* priority array */}
      {writable
        ? <PriorityArray p={p} />
        : <div className="card" style={{ padding: 18, display: 'flex', alignItems: 'center', gap: 12, color: 'var(--muted-foreground)' }}>
            <Icon name="lock" size={18} /> <span style={{ fontSize: 13 }}>Sensors are read-only — no command priority array. Commission a <span className="mono">cmd</span> or <span className="mono">sp</span> point to write.</span>
          </div>}
    </div>
  );
}

function EffectiveSource({ p }) {
  if (!p.pa) return <span className="muted">—</span>;
  const idx = p.pa.slots.findIndex(s => s);
  const slot = p.pa.slots[idx];
  if (!slot) return <div style={{ fontSize: 13 }}>Relinquish default <b>{p.pa.def}</b></div>;
  return (
    <div>
      <div style={{ display: 'flex', alignItems: 'center', gap: 7 }}>
        <span className="badge badge-primary">Level {idx + 1}</span>
        <span style={{ fontSize: 13, fontWeight: 500 }}>{slot.who}</span>
      </div>
      <div className="muted" style={{ fontSize: 11.5, marginTop: 5 }}>Lowest occupied level wins</div>
    </div>
  );
}

function PriorityArray({ p }) {
  const [slots, setSlots] = React.useState(p.pa.slots);
  const effIdx = slots.findIndex(s => s);
  const relinquish = (i) => setSlots(s => s.map((v, j) => j === i ? null : v));
  const labels = { 1: 'Manual life-safety', 2: 'Manual override', 5: 'Critical', 6: 'Manual operator', 8: 'Operator', 10: 'Operator setpoint', 13: 'awaken agent', 16: 'Schedule / default' };
  return (
    <div className="card" style={{ overflow: 'hidden' }}>
      <div style={{ padding: '14px 18px', borderBottom: '1px solid var(--border)', display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
        <div>
          <h3 style={{ fontSize: 14.5, display: 'flex', alignItems: 'center', gap: 8 }}><Icon name="binary" size={16} /> Priority Array</h3>
          <p className="muted" style={{ fontSize: 12, marginTop: 2 }}>BACnet 16-level command arbitration · level 1 wins, operator always beats agent</p>
        </div>
        <button className="btn btn-primary btn-sm"><Icon name="square-pen" size={14} /> Command</button>
      </div>
      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(2, 1fr)' }}>
        {slots.map((slot, i) => {
          const lvl = i + 1; const isEff = i === effIdx;
          const agent = slot && /agent/.test(slot.who);
          return (
            <div key={i} style={{
              display: 'flex', alignItems: 'center', gap: 11, padding: '9px 16px',
              borderBottom: '1px solid var(--border)', borderRight: i % 2 === 0 ? '1px solid var(--border)' : 'none',
              background: isEff ? 'color-mix(in oklch, var(--primary) 8%, transparent)' : slot ? 'transparent' : 'var(--subtle)',
            }}>
              <span className="tabular mono" style={{ width: 22, fontSize: 12, fontWeight: 700, color: isEff ? 'var(--primary)' : slot ? 'var(--foreground)' : 'var(--muted-foreground)', textAlign: 'right' }}>{lvl}</span>
              <div style={{ flex: 1, minWidth: 0 }}>
                {slot ? <>
                  <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
                    <span className="tabular" style={{ fontSize: 13.5, fontWeight: 650 }}>{slot.v}<span className="muted" style={{ fontSize: 11, fontWeight: 400 }}> {p.unit}</span></span>
                    {isEff && <span className="badge badge-primary" style={{ height: 16, padding: '0 5px', fontSize: 9.5 }}>effective</span>}
                  </div>
                  <div className="muted" style={{ fontSize: 10.5, display: 'flex', alignItems: 'center', gap: 4, marginTop: 1 }}>
                    {agent && <Icon name="sparkles" size={10} style={{ color: 'var(--primary)' }} />}{slot.who}
                  </div>
                </> : <span className="muted" style={{ fontSize: 11.5 }}>{labels[lvl] || '—'}</span>}
              </div>
              {slot ? <button className="btn btn-ghost btn-sm tt" data-tip="Relinquish" onClick={() => relinquish(i)} style={{ width: 24, height: 24, padding: 0, color: 'var(--muted-foreground)' }}><Icon name="x" size={14} /></button>
                : <span className="mono" style={{ fontSize: 10, color: 'var(--muted-foreground)', opacity: 0.5 }}>null</span>}
            </div>
          );
        })}
      </div>
      <div style={{ display: 'flex', alignItems: 'center', gap: 10, padding: '11px 16px', background: 'var(--subtle)' }}>
        <span className="mono" style={{ fontSize: 11.5, color: 'var(--muted-foreground)' }}>relinquish_default</span>
        <span className="tabular" style={{ fontSize: 12.5, fontWeight: 600 }}>{p.pa.def} {p.unit}</span>
        <span style={{ marginLeft: 'auto', display: 'flex', alignItems: 'center', gap: 6, fontSize: 11.5 }} className="muted">
          <Icon name="hand" size={13} /> AI writes gated above level 13 require approval
        </span>
      </div>
    </div>
  );
}

Object.assign(window, { Points, PointDetail, PriorityArray, PointKindDot });
