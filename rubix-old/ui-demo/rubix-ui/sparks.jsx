/* RUBIX — Sparks / findings inbox */

function Sparks({ setView, openAgent }) {
  const [filter, setFilter] = React.useState('all');
  const [acked, setAcked] = React.useState({});
  const [sel, setSel] = React.useState(SPARKS[0].id);

  const counts = {
    all: SPARKS.length,
    fault: SPARKS.filter(s => s.severity === 'fault').length,
    warning: SPARKS.filter(s => s.severity === 'warning').length,
    info: SPARKS.filter(s => s.severity === 'info').length,
  };
  const list = SPARKS.filter(s => filter === 'all' ? true : filter === 'open' ? !(s.ack || acked[s.id]) : s.severity === filter);
  const active = SPARKS.find(s => s.id === sel) || list[0];

  return (
    <div style={{ flex: 1, display: 'flex', minHeight: 0 }}>
      {/* list column */}
      <div style={{ width: 440, flex: 'none', borderRight: '1px solid var(--border)', display: 'flex', flexDirection: 'column', minHeight: 0 }}>
        <div style={{ padding: '14px 16px', borderBottom: '1px solid var(--border)', display: 'flex', flexDirection: 'column', gap: 12 }}>
          <Tabs value={filter} onChange={setFilter} tabs={[
            { value: 'all', label: 'All', count: counts.all },
            { value: 'fault', label: 'Faults', count: counts.fault },
            { value: 'warning', label: 'Warnings', count: counts.warning },
            { value: 'info', label: 'Info', count: counts.info },
          ]} />
        </div>
        <div className="scroll" style={{ flex: 1, overflowY: 'auto', padding: 8 }}>
          {list.map(s => {
            const isAck = s.ack || acked[s.id];
            const on = s.id === active?.id;
            return (
              <button key={s.id} onClick={() => setSel(s.id)} style={{
                display: 'flex', gap: 11, width: '100%', textAlign: 'left', padding: '12px 11px', borderRadius: 10, alignItems: 'flex-start',
                background: on ? 'var(--accent-bg)' : 'transparent', opacity: isAck ? 0.6 : 1,
                border: '1px solid ' + (on ? 'color-mix(in oklch, var(--primary) 25%, transparent)' : 'transparent'), marginBottom: 2,
              }} onMouseEnter={e => { if (!on) e.currentTarget.style.background = 'var(--subtle)'; }} onMouseLeave={e => { if (!on) e.currentTarget.style.background = 'transparent'; }}>
                <span style={{ marginTop: 1 }}><SevIcon sev={s.severity} size={16} /></span>
                <div style={{ flex: 1, minWidth: 0 }}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: 7, marginBottom: 3, minWidth: 0 }}>
                    <span className="mono" style={{ fontSize: 11, color: 'var(--muted-foreground)', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap', minWidth: 0 }}>{s.rule}</span>
                    {isAck && <Icon name="check-check" size={13} style={{ color: 'var(--positive)', flex: 'none' }} />}
                  </div>
                  <div style={{ fontSize: 12.5, fontWeight: 500, lineHeight: 1.4, display: '-webkit-box', WebkitLineClamp: 2, WebkitBoxOrient: 'vertical', overflow: 'hidden' }}>{s.message}</div>
                  <div className="muted" style={{ fontSize: 11, marginTop: 4, display: 'flex', alignItems: 'center', gap: 6 }}>
                    <Icon name="box" size={12} /> {s.equip} · {s.ts}
                    {s.agent && <span className="badge badge-primary" style={{ height: 16, padding: '0 5px', fontSize: 9.5 }}><Icon name="sparkles" size={10} /> agent</span>}
                  </div>
                </div>
              </button>
            );
          })}
        </div>
      </div>

      {/* detail column */}
      <div className="scroll" style={{ flex: 1, overflowY: 'auto', minWidth: 0 }}>
        {active && <SparkDetail s={active} acked={s => acked[s] || SPARKS.find(x => x.id === s)?.ack} onAck={() => setAcked(a => ({ ...a, [active.id]: true }))} openAgent={openAgent} setView={setView} />}
      </div>
    </div>
  );
}

function SparkDetail({ s, acked, onAck, openAgent, setView }) {
  const isAck = acked(s.id);
  return (
    <div style={{ padding: 24, maxWidth: 820 }}>
      <div style={{ display: 'flex', alignItems: 'flex-start', gap: 14, marginBottom: 20 }}>
        <span style={{ width: 44, height: 44, borderRadius: 12, flex: 'none', display: 'grid', placeItems: 'center' }} className={'bg-sev-' + s.severity}>
          <SevIcon sev={s.severity} size={22} />
        </span>
        <div style={{ flex: 1, minWidth: 0 }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 4 }}>
            <span className={'badge bg-sev-' + s.severity} style={{ textTransform: 'capitalize', flex: 'none' }}>{s.severity}</span>
            <span className="mono muted" style={{ fontSize: 12, whiteSpace: 'nowrap' }}>{s.rule}</span>
          </div>
          <h2 style={{ fontSize: 19, lineHeight: 1.3, letterSpacing: '-0.02em' }}>{s.message}</h2>
          <div className="muted" style={{ fontSize: 12.5, marginTop: 6, display: 'flex', gap: 14 }}>
            <span style={{ display: 'flex', alignItems: 'center', gap: 5 }}><Icon name="building" size={13} /> {s.site}</span>
            <span style={{ display: 'flex', alignItems: 'center', gap: 5 }}><Icon name="box" size={13} /> {s.equip}</span>
            <span style={{ display: 'flex', alignItems: 'center', gap: 5 }}><Icon name="clock" size={13} /> {s.ts}</span>
          </div>
        </div>
      </div>

      <div style={{ display: 'flex', gap: 9, marginBottom: 22, flexWrap: 'wrap' }}>
        <button className="btn btn-primary" onClick={openAgent}><Icon name="sparkles" size={16} /> Diagnose with awaken</button>
        {!isAck
          ? <button className="btn btn-outline" onClick={onAck}><Icon name="check" size={16} /> Acknowledge</button>
          : <span className="btn btn-secondary" style={{ pointerEvents: 'none', color: 'var(--positive)' }}><Icon name="check-check" size={16} /> Acknowledged</span>}
        <button className="btn btn-outline" onClick={() => setView('points')}><Icon name="network" size={16} /> View points</button>
        <button className="btn btn-ghost"><Icon name="users" size={16} /> Assign</button>
      </div>

      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 14, marginBottom: 16 }}>
        <div className="card" style={{ padding: 16 }}>
          <div className="eyebrow" style={{ marginBottom: 10 }}>Implicated points</div>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
            {s.points.map(p => (
              <div key={p} style={{ display: 'flex', alignItems: 'center', gap: 9, padding: '8px 10px', borderRadius: 8, background: 'var(--subtle)', border: '1px solid var(--border)' }}>
                <Icon name="circle-dot" size={14} style={{ color: 'var(--muted-foreground)' }} />
                <span className="mono" style={{ fontSize: 12.5, flex: 1 }}>{p}</span>
                <Sparkline data={series(20, 50, 22, p.length * 7)} w={70} h={22} />
              </div>
            ))}
          </div>
        </div>
        <div className="card" style={{ padding: 16 }}>
          <div className="eyebrow" style={{ marginBottom: 10 }}>Rule context</div>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 10, fontSize: 12.5 }}>
            <Row k="Rule board" v={s.rule} mono />
            <Row k="Severity" v={s.severity} />
            <Row k="Keyexpr" v={`acme/*/spark/${s.rule}/**`} mono />
            <Row k="First seen" v={s.ts} />
            <Row k="Status" v={isAck ? 'Acknowledged' : 'Open'} />
          </div>
        </div>
      </div>

      <div className="card" style={{ padding: 16 }}>
        <div className="eyebrow" style={{ marginBottom: 12 }}>Trend · last 60 min</div>
        <AreaChart h={170} series={[{ data: series(60, 60, 28, s.id.length * 11, { trend: s.severity === 'fault' ? 0.4 : 0 }), color: 'var(--sev-' + s.severity + ')' }]} />
      </div>
    </div>
  );
}

function Row({ k, v, mono }) {
  return <div style={{ display: 'flex', justifyContent: 'space-between', gap: 12 }}>
    <span className="muted">{k}</span>
    <span className={mono ? 'mono' : ''} style={{ fontWeight: 500, textAlign: 'right', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{v}</span>
  </div>;
}

Object.assign(window, { Sparks, SparkDetail, Row });
