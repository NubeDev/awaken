/* RUBIX — Site Dashboard */

function PageWrap({ children, pad = 22 }) {
  return <div className="scroll" style={{ flex: 1, overflowY: 'auto', overflowX: 'hidden' }}>
    <div style={{ padding: pad, maxWidth: 1480, margin: '0 auto' }}>{children}</div>
  </div>;
}

function SevIcon({ sev, size = 14 }) {
  const m = { fault: 'circle-alert', warning: 'triangle-alert', info: 'info' };
  return <Icon name={m[sev]} size={size} className={'sev-' + sev} />;
}

function Dashboard({ activeSite, setView }) {
  const labels = Array.from({ length: 48 }, (_, i) => `${(i % 24).toString().padStart(2, '0')}:00`);
  const eqOnline = EQUIPS.filter(e => e.online).length;
  return (
    <PageWrap>
      {/* KPI row */}
      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', gap: 14, marginBottom: 14 }}>
        <KPI label="Current Demand" value="412" unit="kW" delta="3.4%" deltaDir="down" icon="zap" spark={ENERGY.demand.slice(-16)} sparkColor="var(--chart-1)" sub="vs 426 kW baseline" />
        <KPI label="Energy Today" value="6.8" unit="MWh" delta="1.9%" deltaDir="down" icon="activity" spark={ENERGY.today.slice(-16)} sparkColor="var(--chart-3)" sub="EUI 138 kWh/m²" />
        <KPI label="Comfort Index" value="97.2" unit="%" delta="0.6%" deltaDir="up" icon="thermometer" spark={series(16, 96, 2, 5)} sparkColor="var(--chart-2)" sub="3 zones out of band" />
        <KPI label="Open Sparks" value="3" unit="active" delta="2 new" deltaDir="up" icon="zap" spark={series(16, 4, 3, 8)} sparkColor="var(--chart-4)" sub="1 fault · 2 warnings" />
      </div>

      {/* main grid */}
      <div style={{ display: 'grid', gridTemplateColumns: '1.7fr 1fr', gap: 14, marginBottom: 14 }}>
        <div className="card" style={{ padding: 18 }}>
          <SectionHead title="Demand · 48h" sub="Whole-building electrical load against weather-normalised baseline">
            <Segmented size="sm" options={[{ value: '24h', label: '24h' }, { value: '48h', label: '48h' }, { value: '7d', label: '7d' }]} value="48h" onChange={() => {}} />
          </SectionHead>
          <div style={{ display: 'flex', gap: 16, marginBottom: 10 }}>
            <LegendInline color="var(--chart-1)" label="Actual demand" value="412 kW" />
            <LegendInline color="var(--muted-foreground)" label="Baseline" value="426 kW" dashed />
            <div style={{ marginLeft: 'auto', display: 'flex', alignItems: 'center', gap: 6 }} className="badge bg-sev-info">
              <Icon name="trending-down" size={13} /> 14 kW below baseline
            </div>
          </div>
          <AreaChart h={216} labels={labels} series={[
            { data: ENERGY.baseline, color: 'var(--muted-foreground)', dashed: true },
            { data: ENERGY.demand, color: 'var(--chart-1)' },
          ]} />
        </div>

        <div className="card" style={{ padding: 18 }}>
          <SectionHead title="Load Breakdown" sub="By system · live" />
          <div style={{ display: 'flex', alignItems: 'center', gap: 20 }}>
            <Donut data={LOAD_SPLIT} size={140} />
            <div style={{ flex: 1 }}><Legend items={LOAD_SPLIT} /></div>
          </div>
        </div>
      </div>

      {/* equipment + sparks */}
      <div style={{ display: 'grid', gridTemplateColumns: '1.7fr 1fr', gap: 14 }}>
        <div className="card" style={{ padding: 18 }}>
          <SectionHead title="Equipment Health" sub={`${eqOnline}/${EQUIPS.length} online · ${activeSite.name}`}>
            <button className="btn btn-outline btn-sm" onClick={() => setView('points')}>Open browser <Icon name="arrow-right" size={14} /></button>
          </SectionHead>
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: 10 }}>
            {EQUIPS.slice(0, 9).map(e => <EquipTile key={e.id} e={e} onClick={() => setView('points')} />)}
          </div>
        </div>

        <div className="card" style={{ padding: 18, display: 'flex', flexDirection: 'column' }}>
          <SectionHead title="Recent Sparks" sub="Latest rule findings">
            <button className="btn btn-ghost btn-sm" onClick={() => setView('sparks')}>All</button>
          </SectionHead>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 2, flex: 1 }}>
            {SPARKS.slice(0, 5).map(s => (
              <button key={s.id} onClick={() => setView('sparks')} style={{
                display: 'flex', gap: 11, padding: '10px 9px', borderRadius: 9, textAlign: 'left', alignItems: 'flex-start',
              }} onMouseEnter={e => e.currentTarget.style.background = 'var(--accent-bg)'} onMouseLeave={e => e.currentTarget.style.background = 'transparent'}>
                <span style={{ marginTop: 1 }}><SevIcon sev={s.severity} size={15} /></span>
                <div style={{ flex: 1, minWidth: 0 }}>
                  <div style={{ fontSize: 12.5, fontWeight: 500, lineHeight: 1.4, display: '-webkit-box', WebkitLineClamp: 2, WebkitBoxOrient: 'vertical', overflow: 'hidden' }}>{s.message}</div>
                  <div className="muted" style={{ fontSize: 11, marginTop: 3, display: 'flex', gap: 7, alignItems: 'center' }}>
                    <span style={{ fontWeight: 600 }}>{s.equip}</span> · {s.ts}
                    {s.agent && <span className="badge badge-primary" style={{ height: 16, padding: '0 5px', fontSize: 9.5 }}><Icon name="sparkles" size={10} /> agent</span>}
                  </div>
                </div>
              </button>
            ))}
          </div>
        </div>
      </div>
    </PageWrap>
  );
}

function LegendInline({ color, label, value, dashed }) {
  return (
    <div style={{ display: 'flex', alignItems: 'center', gap: 7 }}>
      <span style={{ width: 14, height: 0, borderTop: `2px ${dashed ? 'dashed' : 'solid'} ${color}` }} />
      <span className="muted" style={{ fontSize: 12 }}>{label}</span>
      <span className="tabular" style={{ fontSize: 12, fontWeight: 600 }}>{value}</span>
    </div>
  );
}

function EquipTile({ e, onClick }) {
  return (
    <button onClick={onClick} className="focusable" style={{
      display: 'flex', alignItems: 'center', gap: 11, padding: '11px 12px', borderRadius: 10, textAlign: 'left',
      border: '1px solid var(--border)', background: 'var(--subtle)', transition: 'border-color .14s, background .14s',
    }} onMouseEnter={ev => { ev.currentTarget.style.borderColor = 'var(--border-strong)'; ev.currentTarget.style.background = 'var(--accent-bg)'; }}
      onMouseLeave={ev => { ev.currentTarget.style.borderColor = 'var(--border)'; ev.currentTarget.style.background = 'var(--subtle)'; }}>
      <span style={{ width: 34, height: 34, borderRadius: 9, flex: 'none', display: 'grid', placeItems: 'center', background: 'var(--card)', border: '1px solid var(--border)', color: e.alarm ? 'var(--sev-fault)' : 'var(--muted-foreground)' }}>
        <Icon name={e.icon} size={17} />
      </span>
      <div style={{ flex: 1, minWidth: 0 }}>
        <div style={{ fontSize: 12.5, fontWeight: 600, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{e.name}</div>
        <div className="muted" style={{ fontSize: 11, display: 'flex', alignItems: 'center', gap: 5 }}>
          <span style={{ width: 6, height: 6, borderRadius: 99, background: e.alarm ? 'var(--sev-fault)' : 'var(--positive)' }} />
          {e.alarm ? 'Fault active' : 'Nominal'} · {e.points} pts
        </div>
      </div>
    </button>
  );
}

Object.assign(window, { Dashboard, PageWrap, SevIcon, EquipTile });
