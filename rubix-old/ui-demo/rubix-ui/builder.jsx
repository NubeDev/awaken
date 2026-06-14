/* RUBIX — Dashboard Builder (drag/drop grid + palette + binding) */

const WIDGET_TYPES = [
  { id: 'kpi', label: 'KPI Stat', icon: 'gauge-circle', desc: 'Single live value + trend' },
  { id: 'line', label: 'Line / Area', icon: 'line-chart', desc: 'Time-series history' },
  { id: 'bar', label: 'Bar Chart', icon: 'bar-chart', desc: 'Compare across equipment' },
  { id: 'donut', label: 'Donut', icon: 'pie-chart', desc: 'Composition / load split' },
  { id: 'gauge', label: 'Gauge', icon: 'gauge', desc: 'Value vs range' },
  { id: 'table', label: 'Table', icon: 'table', desc: 'Query result grid' },
  { id: 'sparks', label: 'Spark Feed', icon: 'zap', desc: 'Live rule findings' },
  { id: 'map', label: 'Floor Map', icon: 'map', desc: 'Spatial point overlay' },
];

const TEMPLATES = [
  { id: 't1', name: 'Site Overview', icon: 'gauge', tiles: 8 },
  { id: 't2', name: 'Energy & Demand', icon: 'zap', tiles: 6 },
  { id: 't3', name: 'Chiller Plant', icon: 'droplet', tiles: 11 },
  { id: 't4', name: 'AHU Fleet', icon: 'fan', tiles: 9 },
];

/* default canvas layout: {id, type, x, y, w, h, title, binding} on a 12-col grid */
const INIT_TILES = [
  { id: 'w1', type: 'kpi', x: 0, y: 0, w: 3, h: 2, title: 'Current Demand', binding: 'meter-main/kw-total' },
  { id: 'w2', type: 'kpi', x: 3, y: 0, w: 3, h: 2, title: 'Comfort Index', binding: 'site/comfort' },
  { id: 'w3', type: 'gauge', x: 6, y: 0, w: 3, h: 4, title: 'Chiller-1 Load', binding: 'chiller-1/load' },
  { id: 'w4', type: 'donut', x: 9, y: 0, w: 3, h: 4, title: 'Load Split', binding: 'site/load-split' },
  { id: 'w5', type: 'line', x: 0, y: 2, w: 6, h: 4, title: 'Demand · 48h', binding: 'meter-main/kw/his' },
  { id: 'w6', type: 'bar', x: 0, y: 6, w: 4, h: 4, title: 'Energy by Equip', binding: 'site/energy-rank' },
  { id: 'w7', type: 'sparks', x: 4, y: 6, w: 4, h: 4, title: 'Live Sparks', binding: 'site/spark/**' },
  { id: 'w8', type: 'table', x: 8, y: 4, w: 4, h: 6, title: 'AHU Discharge Temps', binding: 'SELECT…' },
];

const COLS = 12, ROW_H = 64, GAP = 12;

function Builder({ activeSite }) {
  const [tiles, setTiles] = React.useState(INIT_TILES);
  const [sel, setSel] = React.useState('w5');
  const [editing, setEditing] = React.useState(true);
  const [drag, setDrag] = React.useState(null);
  const gridRef = React.useRef(null);

  const cellW = () => { const el = gridRef.current; if (!el) return 90; return (el.clientWidth - GAP * (COLS - 1)) / COLS; };

  const onPointerDown = (e, tile, mode) => {
    if (!editing) return;
    e.preventDefault(); e.stopPropagation();
    const startX = e.clientX, startY = e.clientY; const cw = cellW();
    const orig = { ...tile };
    setSel(tile.id);
    const move = (ev) => {
      const dx = Math.round((ev.clientX - startX) / (cw + GAP));
      const dy = Math.round((ev.clientY - startY) / (ROW_H + GAP));
      setTiles(ts => ts.map(t => {
        if (t.id !== tile.id) return t;
        if (mode === 'move') return { ...t, x: Math.max(0, Math.min(COLS - t.w, orig.x + dx)), y: Math.max(0, orig.y + dy) };
        return { ...t, w: Math.max(2, Math.min(COLS - t.x, orig.w + dx)), h: Math.max(2, orig.h + dy) };
      }));
    };
    const up = () => { window.removeEventListener('pointermove', move); window.removeEventListener('pointerup', up); setDrag(null); };
    setDrag(tile.id);
    window.addEventListener('pointermove', move); window.addEventListener('pointerup', up);
  };

  const addWidget = (type) => {
    const maxY = tiles.reduce((m, t) => Math.max(m, t.y + t.h), 0);
    const wt = WIDGET_TYPES.find(w => w.id === type);
    const nt = { id: 'w' + Date.now(), type, x: 0, y: maxY, w: type === 'kpi' ? 3 : 4, h: type === 'kpi' ? 2 : 4, title: wt.label, binding: '— unbound —' };
    setTiles(ts => [...ts, nt]); setSel(nt.id);
  };
  const removeTile = (id) => setTiles(ts => ts.filter(t => t.id !== id));
  const gridH = tiles.reduce((m, t) => Math.max(m, t.y + t.h), 8) * (ROW_H + GAP);
  const selTile = tiles.find(t => t.id === sel);

  return (
    <div style={{ flex: 1, display: 'flex', minHeight: 0 }}>
      {/* palette */}
      {editing && <div className="scroll" style={{ width: 232, flex: 'none', borderRight: '1px solid var(--border)', overflowY: 'auto', padding: 14 }}>
        <div className="eyebrow" style={{ marginBottom: 10 }}>Widgets</div>
        <div style={{ display: 'flex', flexDirection: 'column', gap: 6, marginBottom: 20 }}>
          {WIDGET_TYPES.map(w => (
            <button key={w.id} onClick={() => addWidget(w.id)} style={{
              display: 'flex', alignItems: 'center', gap: 10, padding: '9px 10px', borderRadius: 9, textAlign: 'left',
              border: '1px solid var(--border)', background: 'var(--subtle)',
            }} onMouseEnter={e => { e.currentTarget.style.borderColor = 'var(--primary)'; e.currentTarget.style.background = 'var(--accent-bg)'; }}
              onMouseLeave={e => { e.currentTarget.style.borderColor = 'var(--border)'; e.currentTarget.style.background = 'var(--subtle)'; }}>
              <span style={{ width: 30, height: 30, borderRadius: 7, flex: 'none', display: 'grid', placeItems: 'center', background: 'var(--card)', border: '1px solid var(--border)', color: 'var(--primary)' }}><Icon name={w.icon} size={16} /></span>
              <div style={{ minWidth: 0 }}>
                <div style={{ fontSize: 12.5, fontWeight: 600 }}>{w.label}</div>
                <div className="muted" style={{ fontSize: 10.5, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{w.desc}</div>
              </div>
              <Icon name="plus" size={15} style={{ color: 'var(--muted-foreground)', marginLeft: 'auto' }} />
            </button>
          ))}
        </div>
        <div className="eyebrow" style={{ marginBottom: 10 }}>Start from template</div>
        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 6 }}>
          {TEMPLATES.map(t => (
            <button key={t.id} style={{ padding: '12px 8px', borderRadius: 9, border: '1px solid var(--border)', display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 6, background: 'var(--subtle)' }}
              onMouseEnter={e => e.currentTarget.style.borderColor = 'var(--primary)'} onMouseLeave={e => e.currentTarget.style.borderColor = 'var(--border)'}>
              <Icon name={t.icon} size={17} style={{ color: 'var(--primary)' }} />
              <span style={{ fontSize: 11, fontWeight: 600, textAlign: 'center' }}>{t.name}</span>
              <span className="muted" style={{ fontSize: 10 }}>{t.tiles} tiles</span>
            </button>
          ))}
        </div>
      </div>}

      {/* canvas */}
      <div style={{ flex: 1, display: 'flex', flexDirection: 'column', minWidth: 0 }}>
        <div style={{ height: 50, flex: 'none', borderBottom: '1px solid var(--border)', display: 'flex', alignItems: 'center', gap: 10, padding: '0 14px' }}>
          <input defaultValue="Site Overview" className="focusable" style={{ fontSize: 14, fontWeight: 600, background: 'none', border: 'none', outline: 'none', width: 138, borderRadius: 6, padding: '4px 6px', flex: 'none' }} />
          <span className="badge badge-muted" style={{ flex: 'none' }}>{tiles.length} widgets</span>
          <span className="muted tt" data-tip="Auto-saved" style={{ fontSize: 11.5, display: 'flex', alignItems: 'center', gap: 5, flex: 'none' }}><Icon name="refresh" size={12} /></span>
          <div style={{ marginLeft: 'auto', display: 'flex', alignItems: 'center', gap: 8, flex: 'none' }}>
            <button className="btn btn-ghost btn-icon btn-sm tt" data-tip="Build with AI"><Icon name="sparkles" size={15} style={{ color: 'var(--primary)' }} /></button>
            <Segmented size="sm" value={editing ? 'edit' : 'view'} onChange={v => setEditing(v === 'edit')} options={[{ value: 'edit', label: 'Edit', icon: 'grip' }, { value: 'view', label: 'Preview', icon: 'eye' }]} />
            <button className="btn btn-primary btn-sm"><Icon name="check" size={14} /> Publish</button>
          </div>
        </div>

        <div className="scroll" ref={gridRef} style={{ flex: 1, overflowY: 'auto', padding: 16, position: 'relative', background: editing ? 'var(--subtle)' : 'var(--background)' }}>
          {/* grid backdrop */}
          {editing && <div style={{ position: 'absolute', inset: 16, backgroundImage: `linear-gradient(var(--grid-line) 1px, transparent 1px), linear-gradient(90deg, var(--grid-line) 1px, transparent 1px)`, backgroundSize: `${100 / COLS}% ${ROW_H + GAP}px`, opacity: 0.5, pointerEvents: 'none', borderRadius: 8 }} />}
          <div style={{ position: 'relative', height: gridH }}>
            {tiles.map(t => (
              <BuilderTile key={t.id} t={t} cols={COLS} rowH={ROW_H} gap={GAP} editing={editing} selected={sel === t.id} dragging={drag === t.id}
                onSelect={() => setSel(t.id)} onMove={e => onPointerDown(e, t, 'move')} onResize={e => onPointerDown(e, t, 'resize')} onRemove={() => removeTile(t.id)} />
            ))}
          </div>
        </div>
      </div>

      {/* inspector */}
      {editing && <div className="scroll" style={{ width: 268, flex: 'none', borderLeft: '1px solid var(--border)', overflowY: 'auto' }}>
        {selTile ? <Inspector t={selTile} onChange={patch => setTiles(ts => ts.map(x => x.id === selTile.id ? { ...x, ...patch } : x))} /> :
          <div style={{ padding: 24, textAlign: 'center', color: 'var(--muted-foreground)', fontSize: 13 }}>Select a widget to bind data and edit its properties.</div>}
      </div>}
    </div>
  );
}

function BuilderTile({ t, cols, rowH, gap, editing, selected, dragging, onSelect, onMove, onResize, onRemove }) {
  const style = {
    position: 'absolute', left: `calc(${(t.x / cols) * 100}% + ${t.x ? gap / 2 : 0}px)`, width: `calc(${(t.w / cols) * 100}% - ${gap}px)`,
    top: t.y * (rowH + gap), height: t.h * (rowH + gap) - gap, transition: dragging ? 'none' : 'left .12s, top .12s, width .12s, height .12s',
    zIndex: dragging ? 30 : selected ? 20 : 1,
  };
  return (
    <div style={style} onPointerDown={editing ? onMove : undefined} onClick={onSelect}>
      <div className="card" style={{
        height: '100%', padding: 14, display: 'flex', flexDirection: 'column', cursor: editing ? (dragging ? 'grabbing' : 'grab') : 'default', overflow: 'hidden',
        boxShadow: dragging ? 'var(--shadow-xl)' : selected && editing ? '0 0 0 2px var(--ring), var(--shadow-md)' : 'var(--shadow-sm)',
        borderColor: selected && editing ? 'transparent' : 'var(--border)',
      }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 8 }}>
          {editing && <Icon name="grip-vertical" size={14} style={{ color: 'var(--muted-foreground)', margin: '0 -4px 0 -4px' }} />}
          <span style={{ fontSize: 12.5, fontWeight: 600, flex: 1, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{t.title}</span>
          {editing && <button onClick={(e) => { e.stopPropagation(); onRemove(); }} onPointerDown={e => e.stopPropagation()} className="btn btn-ghost" style={{ width: 22, height: 22, padding: 0, color: 'var(--muted-foreground)' }}><Icon name="x" size={13} /></button>}
        </div>
        <div style={{ flex: 1, minHeight: 0, display: 'flex', flexDirection: 'column', justifyContent: 'center', pointerEvents: 'none' }}>
          <TilePreview t={t} />
        </div>
        {editing && <div onPointerDown={(e) => { e.stopPropagation(); onResize(e); }} style={{ position: 'absolute', right: 2, bottom: 2, width: 16, height: 16, cursor: 'nwse-resize', color: 'var(--muted-foreground)', display: 'grid', placeItems: 'center' }}>
          <svg width="10" height="10" viewBox="0 0 10 10"><path d="M9 1L1 9M9 5L5 9" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" /></svg>
        </div>}
      </div>
    </div>
  );
}

function TilePreview({ t }) {
  const seed = t.id.length * 13 + t.x;
  if (t.type === 'kpi') return (
    <div>
      <div style={{ display: 'flex', alignItems: 'baseline', gap: 4 }}><span className="tabular" style={{ fontSize: 26, fontWeight: 700, letterSpacing: '-0.03em' }}>{t.title.includes('Comfort') ? '97.2' : '412'}</span><span className="muted" style={{ fontSize: 12 }}>{t.title.includes('Comfort') ? '%' : 'kW'}</span></div>
      <div style={{ marginTop: 6 }}><Sparkline data={series(20, 50, 16, seed)} w={140} h={28} /></div>
    </div>
  );
  if (t.type === 'line') return <AreaChart h={Math.max(90, t.h * 56 - 64)} series={[{ data: series(40, 50, 22, seed), color: 'var(--chart-1)' }]} />;
  if (t.type === 'bar') return <BarChart h={120} data={series(8, 40, 30, seed).map(Math.abs)} color="var(--chart-2)" />;
  if (t.type === 'donut') return <div style={{ display: 'grid', placeItems: 'center', height: '100%' }}><Donut data={LOAD_SPLIT} size={92} thickness={13} /></div>;
  if (t.type === 'gauge') return <div style={{ display: 'grid', placeItems: 'center', height: '100%' }}><Gauge value={72} size={104} color="var(--chart-1)" label="load" /></div>;
  if (t.type === 'sparks') return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
      {SPARKS.slice(0, 3).map(s => <div key={s.id} style={{ display: 'flex', gap: 7, alignItems: 'center', fontSize: 11 }}><SevIcon sev={s.severity} size={12} /><span style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap', color: 'var(--muted-foreground)' }}>{s.equip} · {s.rule}</span></div>)}
    </div>
  );
  if (t.type === 'table') return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 0, fontSize: 11 }}>
      {['AHU-1', 'AHU-3', 'AHU-5', 'AHU-7'].map((r, i) => <div key={r} style={{ display: 'flex', justifyContent: 'space-between', gap: 8, padding: '5px 0', borderBottom: '1px solid var(--border)' }}><span className="mono muted" style={{ whiteSpace: 'nowrap' }}>{r}</span><span className="tabular" style={{ fontWeight: 600, whiteSpace: 'nowrap', color: i === 1 ? 'var(--sev-fault)' : 'inherit' }}>{(13 + i * 0.4).toFixed(1)}°C</span></div>)}
    </div>
  );
  if (t.type === 'map') return <Placeholder label="floor-plan.svg" h={'100%'} icon="map" />;
  return null;
}

function Inspector({ t, onChange }) {
  const wt = WIDGET_TYPES.find(w => w.id === t.type);
  return (
    <div style={{ padding: 16 }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 9, marginBottom: 16 }}>
        <span style={{ width: 30, height: 30, borderRadius: 8, display: 'grid', placeItems: 'center', background: 'var(--accent-bg)', color: 'var(--primary)' }}><Icon name={wt.icon} size={16} /></span>
        <div><div style={{ fontSize: 13, fontWeight: 600 }}>{wt.label}</div><div className="muted mono" style={{ fontSize: 10.5 }}>{t.id}</div></div>
      </div>

      <Field label="Title">
        <input value={t.title} onChange={e => onChange({ title: e.target.value })} className="input" style={{ height: 32, fontSize: 12.5 }} />
      </Field>

      <Field label="Data binding">
        <div className="input" style={{ height: 32, padding: '0 8px', gap: 6 }}>
          <Icon name="database" size={13} style={{ color: 'var(--primary)' }} />
          <input value={t.binding} onChange={e => onChange({ binding: e.target.value })} className="mono" style={{ flex: 1, background: 'none', border: 'none', outline: 'none', fontSize: 11 }} />
        </div>
        <div className="muted" style={{ fontSize: 10.5, marginTop: 5, display: 'flex', alignItems: 'center', gap: 5 }}>
          <span className="live-dot" /> Live via zenoh subscription
        </div>
      </Field>

      <Field label="Source type">
        <Segmented size="sm" value="point" onChange={() => {}} options={[{ value: 'point', label: 'Point' }, { value: 'sql', label: 'SQL' }, { value: 'board', label: 'Board' }]} />
      </Field>

      <Field label="Refresh">
        <Segmented size="sm" value="live" onChange={() => {}} options={['live', '10s', '1m', '5m']} />
      </Field>

      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 8, marginBottom: 16 }}>
        <Field label="Width" inline><div className="input" style={{ height: 30, justifyContent: 'space-between' }}><span className="tabular" style={{ fontSize: 12 }}>{t.w}</span><span className="muted" style={{ fontSize: 10 }}>cols</span></div></Field>
        <Field label="Height" inline><div className="input" style={{ height: 30, justifyContent: 'space-between' }}><span className="tabular" style={{ fontSize: 12 }}>{t.h}</span><span className="muted" style={{ fontSize: 10 }}>rows</span></div></Field>
      </div>

      <div className="sep-x" style={{ margin: '4px 0 14px' }} />
      <button className="btn btn-outline btn-sm" style={{ width: '100%' }}><Icon name="copy" size={14} /> Duplicate widget</button>
    </div>
  );
}

function Field({ label, children, inline }) {
  return <div style={{ marginBottom: inline ? 0 : 14 }}>
    <div className="eyebrow" style={{ fontSize: 10, marginBottom: 6 }}>{label}</div>
    {children}
  </div>;
}

Object.assign(window, { Builder });
