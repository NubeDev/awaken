/* RUBIX — Flow Boards (react-flow-style node canvas) */

const NODE_W = 188;
const KIND_STYLE = {
  in: { color: 'var(--chart-2)', label: 'Source' },
  logic: { color: 'var(--chart-5)', label: 'Logic' },
  out: { color: 'var(--chart-1)', label: 'Write' },
  agent: { color: 'var(--primary)', label: 'AI' },
};

function Flows() {
  const [nodes, setNodes] = React.useState(() => FLOW.nodes.map(n => ({ ...n })));
  const [sel, setSel] = React.useState('n5');
  const [pan, setPan] = React.useState({ x: 40, y: 20 });
  const [zoom, setZoom] = React.useState(1);
  const [dragNode, setDragNode] = React.useState(null);
  const wrapRef = React.useRef(null);

  const nodeH = (n) => 56 + Math.max(n.ins?.length || 0, n.outs?.length || 0) * 4;

  const startNodeDrag = (e, node) => {
    e.stopPropagation(); setSel(node.id);
    const sx = e.clientX, sy = e.clientY; const ox = node.x, oy = node.y;
    setDragNode(node.id);
    const move = (ev) => {
      const dx = (ev.clientX - sx) / zoom, dy = (ev.clientY - sy) / zoom;
      setNodes(ns => ns.map(n => n.id === node.id ? { ...n, x: ox + dx, y: oy + dy } : n));
    };
    const up = () => { window.removeEventListener('pointermove', move); window.removeEventListener('pointerup', up); setDragNode(null); };
    window.addEventListener('pointermove', move); window.addEventListener('pointerup', up);
  };

  const startPan = (e) => {
    if (e.target.closest('[data-node]')) return;
    const sx = e.clientX, sy = e.clientY; const o = { ...pan };
    const move = (ev) => setPan({ x: o.x + (ev.clientX - sx), y: o.y + (ev.clientY - sy) });
    const up = () => { window.removeEventListener('pointermove', move); window.removeEventListener('pointerup', up); };
    window.addEventListener('pointermove', move); window.addEventListener('pointerup', up);
  };

  const portPos = (node, type, name) => {
    const list = type === 'out' ? node.outs : node.ins; const i = list.indexOf(name); const n = list.length;
    const h = nodeH(node);
    return { x: node.x + (type === 'out' ? NODE_W : 0), y: node.y + 34 + (i + 0.5) * ((h - 34) / n) };
  };

  const edges = FLOW.edges.map(([a, b]) => {
    const na = nodes.find(n => n.id === a), nb = nodes.find(n => n.id === b);
    const p1 = portPos(na, 'out', na.outs[0]);
    const inName = nb.ins.find(x => na.outs.includes(x)) || nb.ins[0];
    const p2 = portPos(nb, 'in', inName);
    return { a, b, p1, p2 };
  });

  const selNode = nodes.find(n => n.id === sel);

  return (
    <div style={{ flex: 1, display: 'flex', minHeight: 0 }}>
      {/* node palette */}
      <div className="scroll" style={{ width: 220, flex: 'none', borderRight: '1px solid var(--border)', overflowY: 'auto', padding: 14 }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 14 }}>
          <Icon name="workflow" size={16} style={{ color: 'var(--primary)' }} />
          <div><div style={{ fontSize: 13, fontWeight: 600 }}>{FLOW.name}</div><div className="muted" style={{ fontSize: 10.5 }}>control board · v4</div></div>
        </div>
        {NODE_PALETTE.map(g => (
          <div key={g.group} style={{ marginBottom: 14 }}>
            <div className="eyebrow" style={{ fontSize: 10, marginBottom: 7 }}>{g.group}</div>
            <div style={{ display: 'flex', flexDirection: 'column', gap: 5 }}>
              {g.items.map(it => (
                <div key={it.label} draggable style={{
                  display: 'flex', alignItems: 'center', gap: 9, padding: '7px 9px', borderRadius: 8,
                  border: '1px solid var(--border)', background: 'var(--subtle)', cursor: 'grab', fontSize: 12.5,
                }} onMouseEnter={e => e.currentTarget.style.borderColor = 'var(--border-strong)'} onMouseLeave={e => e.currentTarget.style.borderColor = 'var(--border)'}>
                  <Icon name={it.icon} size={15} style={{ color: 'var(--muted-foreground)' }} />
                  <span style={{ flex: 1 }}>{it.label}</span>
                  <Icon name="grip-vertical" size={13} style={{ color: 'var(--muted-foreground)', opacity: 0.6 }} />
                </div>
              ))}
            </div>
          </div>
        ))}
      </div>

      {/* canvas */}
      <div style={{ flex: 1, display: 'flex', flexDirection: 'column', minWidth: 0 }}>
        <div style={{ height: 48, flex: 'none', borderBottom: '1px solid var(--border)', display: 'flex', alignItems: 'center', gap: 10, padding: '0 14px' }}>
          <span className="badge bg-sev-info" style={{ gap: 5 }}><span className="live-dot" /> deployed · running</span>
          <span className="muted" style={{ fontSize: 11.5 }}>{nodes.length} nodes · {edges.length} wires · last tick 1.2s ago</span>
          <div style={{ marginLeft: 'auto', display: 'flex', alignItems: 'center', gap: 8 }}>
            <button className="btn btn-ghost btn-sm"><Icon name="play" size={13} /> Test run</button>
            <button className="btn btn-ghost btn-sm"><Icon name="history" size={14} /> Versions</button>
            <button className="btn btn-primary btn-sm"><Icon name="git-branch" size={14} /> Deploy</button>
          </div>
        </div>

        <div ref={wrapRef} onPointerDown={startPan} style={{
          flex: 1, position: 'relative', overflow: 'hidden', cursor: 'grab',
          background: 'var(--subtle)',
          backgroundImage: `radial-gradient(var(--grid-line) 1.2px, transparent 1.2px)`,
          backgroundSize: `${22 * zoom}px ${22 * zoom}px`, backgroundPosition: `${pan.x}px ${pan.y}px`,
        }}>
          <div style={{ position: 'absolute', left: 0, top: 0, transform: `translate(${pan.x}px, ${pan.y}px) scale(${zoom})`, transformOrigin: '0 0' }}>
            {/* edges */}
            <svg style={{ position: 'absolute', left: 0, top: 0, overflow: 'visible', pointerEvents: 'none', width: 1, height: 1 }}>
              <defs>
                <marker id="arrow" markerWidth="8" markerHeight="8" refX="6" refY="3" orient="auto"><path d="M0,0 L6,3 L0,6" fill="none" stroke="var(--border-strong)" strokeWidth="1.4" /></marker>
              </defs>
              {edges.map((e, i) => {
                const mx = (e.p1.x + e.p2.x) / 2;
                const active = e.a === sel || e.b === sel;
                return <path key={i} d={`M ${e.p1.x} ${e.p1.y} C ${mx} ${e.p1.y}, ${mx} ${e.p2.y}, ${e.p2.x} ${e.p2.y}`}
                  fill="none" stroke={active ? 'var(--primary)' : 'var(--border-strong)'} strokeWidth={active ? 2.2 : 1.6} markerEnd="url(#arrow)"
                  style={{ transition: 'stroke .15s' }} />;
              })}
              {/* animated flow dot on active edge */}
              {edges.filter(e => e.a === sel || e.b === sel).map((e, i) => {
                const mx = (e.p1.x + e.p2.x) / 2;
                return <circle key={'d' + i} r="3" fill="var(--primary)"><animateMotion dur="1.4s" repeatCount="indefinite" path={`M ${e.p1.x} ${e.p1.y} C ${mx} ${e.p1.y}, ${mx} ${e.p2.y}, ${e.p2.x} ${e.p2.y}`} /></circle>;
              })}
            </svg>
            {/* nodes */}
            {nodes.map(n => <FlowNode key={n.id} n={n} h={nodeH(n)} selected={sel === n.id} dragging={dragNode === n.id} onDown={e => startNodeDrag(e, n)} />)}
          </div>

          {/* zoom controls */}
          <div style={{ position: 'absolute', left: 14, bottom: 14, display: 'flex', flexDirection: 'column', gap: 4, background: 'var(--card)', border: '1px solid var(--border)', borderRadius: 9, padding: 4, boxShadow: 'var(--shadow-md)' }}>
            <button className="btn btn-ghost btn-icon btn-sm" onClick={() => setZoom(z => Math.min(1.6, z + 0.15))}><Icon name="plus" size={15} /></button>
            <button className="btn btn-ghost btn-icon btn-sm" onClick={() => setZoom(z => Math.max(0.5, z - 0.15))}><Icon name="minus" size={15} /></button>
            <button className="btn btn-ghost btn-icon btn-sm" onClick={() => { setZoom(1); setPan({ x: 40, y: 20 }); }}><Icon name="scan" size={15} /></button>
          </div>
          <div style={{ position: 'absolute', right: 14, bottom: 14, fontSize: 11 }} className="badge badge-outline">{Math.round(zoom * 100)}%</div>
        </div>
      </div>

      {/* node inspector */}
      <div className="scroll" style={{ width: 256, flex: 'none', borderLeft: '1px solid var(--border)', overflowY: 'auto' }}>
        {selNode && <NodeInspector n={selNode} />}
      </div>
    </div>
  );
}

function FlowNode({ n, h, selected, dragging, onDown }) {
  const ks = KIND_STYLE[n.kind];
  return (
    <div data-node onPointerDown={onDown} style={{
      position: 'absolute', left: n.x, top: n.y, width: NODE_W, cursor: dragging ? 'grabbing' : 'grab',
      borderRadius: 11, background: 'var(--card)', border: '1px solid ' + (selected ? 'transparent' : 'var(--border)'),
      boxShadow: dragging ? 'var(--shadow-xl)' : selected ? '0 0 0 2px var(--ring), var(--shadow-md)' : 'var(--shadow-md)',
      zIndex: dragging || selected ? 10 : 1, userSelect: 'none', transition: dragging ? 'none' : 'box-shadow .15s',
    }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 8, padding: '8px 11px', borderBottom: '1px solid var(--border)' }}>
        <span style={{ width: 24, height: 24, borderRadius: 7, flex: 'none', display: 'grid', placeItems: 'center', background: `color-mix(in oklch, ${ks.color} 16%, transparent)`, color: ks.color }}><Icon name={n.icon} size={14} /></span>
        <div style={{ flex: 1, minWidth: 0 }}>
          <div style={{ fontSize: 12.5, fontWeight: 600, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{n.title}</div>
        </div>
        <span style={{ width: 6, height: 6, borderRadius: 99, background: ks.color, flex: 'none' }} />
      </div>
      <div style={{ padding: '8px 11px', display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
        <span className="mono" style={{ fontSize: 10.5, color: 'var(--muted-foreground)' }}>{n.sub}</span>
        <span className="badge badge-outline" style={{ height: 16, padding: '0 5px', fontSize: 9.5 }}>{ks.label}</span>
      </div>
      {/* ports */}
      {(n.ins || []).map((p, i) => {
        const n_ = (n.ins.length); return (
          <span key={'i' + p} style={{ position: 'absolute', left: -5, top: 34 + (i + 0.5) * ((h - 34) / n_) - 5, width: 10, height: 10, borderRadius: 99, background: 'var(--card)', border: '2px solid var(--border-strong)' }} />
        );
      })}
      {(n.outs || []).map((p, i) => {
        const n_ = (n.outs.length); return (
          <span key={'o' + p} style={{ position: 'absolute', right: -5, top: 34 + (i + 0.5) * ((h - 34) / n_) - 5, width: 10, height: 10, borderRadius: 99, background: KIND_STYLE[n.kind].color, border: '2px solid var(--card)' }} />
        );
      })}
    </div>
  );
}

function NodeInspector({ n }) {
  const ks = KIND_STYLE[n.kind];
  return (
    <div style={{ padding: 16 }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 10, marginBottom: 16 }}>
        <span style={{ width: 34, height: 34, borderRadius: 9, display: 'grid', placeItems: 'center', background: `color-mix(in oklch, ${ks.color} 16%, transparent)`, color: ks.color }}><Icon name={n.icon} size={17} /></span>
        <div><div style={{ fontSize: 13.5, fontWeight: 600 }}>{n.title}</div><div className="muted mono" style={{ fontSize: 10.5 }}>{n.sub}</div></div>
      </div>

      <div className="eyebrow" style={{ fontSize: 10, marginBottom: 7 }}>Node type</div>
      <div className="card" style={{ padding: '9px 11px', marginBottom: 16, display: 'flex', alignItems: 'center', gap: 9, background: 'var(--subtle)' }}>
        <span style={{ width: 8, height: 8, borderRadius: 99, background: ks.color }} />
        <span style={{ fontSize: 12.5, fontWeight: 500 }}>{ks.label} actor</span>
        <span className="mono badge badge-outline" style={{ marginLeft: 'auto', fontSize: 10 }}>#[actor]</span>
      </div>

      {n.kind === 'out' && <div className="card" style={{ padding: 12, marginBottom: 14, background: 'color-mix(in oklch, var(--sev-warning) 8%, transparent)', borderColor: 'color-mix(in oklch, var(--sev-warning) 30%, transparent)' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 7, fontSize: 12, fontWeight: 600, marginBottom: 5 }}><Icon name="hand" size={14} className="sev-warning" /> Gated write</div>
        <div className="muted" style={{ fontSize: 11.5, lineHeight: 1.5 }}>Writes enter the priority array at level 13. Above-threshold writes suspend for HITL approval.</div>
      </div>}

      <div className="eyebrow" style={{ fontSize: 10, marginBottom: 7 }}>Ports</div>
      <div style={{ display: 'flex', flexDirection: 'column', gap: 6, marginBottom: 16 }}>
        {(n.ins || []).map(p => <PortRow key={p} name={p} dir="in" />)}
        {(n.outs || []).map(p => <PortRow key={p} name={p} dir="out" color={ks.color} />)}
      </div>

      <button className="btn btn-outline btn-sm" style={{ width: '100%' }}><Icon name="settings" size={14} /> Configure actor</button>
    </div>
  );
}

function PortRow({ name, dir, color }) {
  return (
    <div style={{ display: 'flex', alignItems: 'center', gap: 9, padding: '6px 9px', borderRadius: 7, background: 'var(--subtle)', border: '1px solid var(--border)' }}>
      <span style={{ width: 8, height: 8, borderRadius: 99, background: dir === 'out' ? color : 'var(--border-strong)' }} />
      <span className="mono" style={{ fontSize: 11.5, flex: 1 }}>{name}</span>
      <span className="muted" style={{ fontSize: 10.5 }}>{dir === 'out' ? 'output' : 'input'}</span>
    </div>
  );
}

Object.assign(window, { Flows });
