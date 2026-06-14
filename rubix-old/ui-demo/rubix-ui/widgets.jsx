/* RUBIX — chart primitives & dashboard widgets (hand-built SVG) */

function pathFrom(data, w, h, pad = 2) {
  const min = Math.min(...data), max = Math.max(...data), range = max - min || 1;
  const dx = (w - pad * 2) / (data.length - 1);
  return data.map((v, i) => `${i === 0 ? 'M' : 'L'} ${(pad + i * dx).toFixed(1)} ${(pad + (h - pad * 2) * (1 - (v - min) / range)).toFixed(1)}`).join(' ');
}

function Sparkline({ data, w = 120, h = 32, color = 'var(--primary)', fill = true, strokeWidth = 1.6 }) {
  const id = React.useMemo(() => 'sg' + Math.random().toString(36).slice(2, 8), []);
  const d = pathFrom(data, w, h, 2);
  const area = d + ` L ${w - 2} ${h - 2} L 2 ${h - 2} Z`;
  return (
    <svg width={w} height={h} viewBox={`0 0 ${w} ${h}`} style={{ overflow: 'visible', display: 'block' }} preserveAspectRatio="none">
      {fill && <defs><linearGradient id={id} x1="0" y1="0" x2="0" y2="1">
        <stop offset="0%" stopColor={color} stopOpacity="0.22" /><stop offset="100%" stopColor={color} stopOpacity="0" />
      </linearGradient></defs>}
      {fill && <path d={area} fill={`url(#${id})`} />}
      <path d={d} fill="none" stroke={color} strokeWidth={strokeWidth} strokeLinecap="round" strokeLinejoin="round" vectorEffect="non-scaling-stroke" />
    </svg>
  );
}

/* Area chart with axis + gridlines */
function AreaChart({ series: ser, w = 600, h = 200, labels, yUnit = '', showBaseline }) {
  const all = ser.flatMap(s => s.data);
  const min = Math.min(...all) * 0.96, max = Math.max(...all) * 1.04, range = max - min || 1;
  const padL = 38, padB = 22, padT = 10, padR = 8;
  const iw = w - padL - padR, ih = h - padT - padB;
  const X = i => padL + (iw) * (i / (ser[0].data.length - 1));
  const Y = v => padT + ih * (1 - (v - min) / range);
  const ticks = 4;
  return (
    <svg width="100%" height={h} viewBox={`0 0 ${w} ${h}`} preserveAspectRatio="none" style={{ display: 'block' }}>
      {Array.from({ length: ticks + 1 }).map((_, i) => {
        const v = min + (range * i / ticks); const y = Y(v);
        return <g key={i}>
          <line x1={padL} y1={y} x2={w - padR} y2={y} stroke="var(--grid-line)" strokeWidth="1" />
          <text x={padL - 7} y={y + 3} textAnchor="end" fontSize="9.5" fill="var(--muted-foreground)" className="tabular">{Math.round(v)}</text>
        </g>;
      })}
      {ser.map((s, si) => {
        const id = 'ac' + si + Math.random().toString(36).slice(2, 6);
        const d = s.data.map((v, i) => `${i === 0 ? 'M' : 'L'} ${X(i).toFixed(1)} ${Y(v).toFixed(1)}`).join(' ');
        const dashed = s.dashed;
        return <g key={si}>
          {!dashed && <><defs><linearGradient id={id} x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" stopColor={s.color} stopOpacity="0.2" /><stop offset="100%" stopColor={s.color} stopOpacity="0" />
          </linearGradient></defs>
          <path d={d + ` L ${X(s.data.length - 1)} ${padT + ih} L ${padL} ${padT + ih} Z`} fill={`url(#${id})`} /></>}
          <path d={d} fill="none" stroke={s.color} strokeWidth={dashed ? 1.5 : 2} strokeDasharray={dashed ? '4 4' : ''} strokeLinejoin="round" strokeLinecap="round" />
        </g>;
      })}
      {labels && labels.map((l, i) => i % Math.ceil(labels.length / 6) === 0 && (
        <text key={i} x={X(i)} y={h - 6} textAnchor="middle" fontSize="9.5" fill="var(--muted-foreground)">{l}</text>
      ))}
    </svg>
  );
}

/* grouped/single bar chart */
function BarChart({ data, w = 600, h = 180, color = 'var(--chart-1)', labels }) {
  const max = Math.max(...data) * 1.1, padB = 20, padT = 6, ih = h - padB - padT;
  const bw = (w / data.length) * 0.56, gap = (w / data.length);
  return (
    <svg width="100%" height={h} viewBox={`0 0 ${w} ${h}`} preserveAspectRatio="none" style={{ display: 'block' }}>
      {data.map((v, i) => {
        const bh = ih * (v / max); const x = i * gap + (gap - bw) / 2;
        return <g key={i}>
          <rect x={x} y={padT + ih - bh} width={bw} height={bh} rx="3" fill={color} opacity={0.35 + 0.65 * (v / max)} />
          {labels && i % Math.ceil(data.length / 8) === 0 && <text x={x + bw / 2} y={h - 5} textAnchor="middle" fontSize="9" fill="var(--muted-foreground)">{labels[i]}</text>}
        </g>;
      })}
    </svg>
  );
}

/* donut for load split */
function Donut({ data, size = 132, thickness = 18, center }) {
  const total = data.reduce((a, b) => a + b.kw, 0);
  const r = (size - thickness) / 2, c = size / 2, circ = 2 * Math.PI * r;
  let off = 0;
  return (
    <div style={{ position: 'relative', width: size, height: size, flex: 'none' }}>
      <svg width={size} height={size} style={{ transform: 'rotate(-90deg)' }}>
        <circle cx={c} cy={c} r={r} fill="none" stroke="var(--muted)" strokeWidth={thickness} />
        {data.map((d, i) => {
          const len = (d.kw / total) * circ;
          const el = <circle key={i} cx={c} cy={c} r={r} fill="none" stroke={d.color} strokeWidth={thickness}
            strokeDasharray={`${len} ${circ - len}`} strokeDashoffset={-off} strokeLinecap="butt" />;
          off += len; return el;
        })}
      </svg>
      <div style={{ position: 'absolute', inset: 0, display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center' }}>
        {center || <><span className="tabular" style={{ fontSize: 22, fontWeight: 650, letterSpacing: '-0.03em' }}>{total}</span><span className="muted" style={{ fontSize: 11 }}>kW total</span></>}
      </div>
    </div>
  );
}

/* radial gauge for a single value */
function Gauge({ value, min = 0, max = 100, unit = '%', size = 120, color = 'var(--primary)', label }) {
  const pct = Math.max(0, Math.min(1, (value - min) / (max - min)));
  const r = (size - 16) / 2, c = size / 2, circ = 2 * Math.PI * r, arc = 0.75; // 270deg
  return (
    <div style={{ position: 'relative', width: size, height: size }}>
      <svg width={size} height={size} style={{ transform: 'rotate(135deg)' }}>
        <circle cx={c} cy={c} r={r} fill="none" stroke="var(--muted)" strokeWidth="9" strokeDasharray={`${circ * arc} ${circ}`} strokeLinecap="round" />
        <circle cx={c} cy={c} r={r} fill="none" stroke={color} strokeWidth="9" strokeDasharray={`${circ * arc * pct} ${circ}`} strokeLinecap="round" style={{ transition: 'stroke-dasharray .5s' }} />
      </svg>
      <div style={{ position: 'absolute', inset: 0, display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center' }}>
        <span className="tabular" style={{ fontSize: size * 0.22, fontWeight: 650, letterSpacing: '-0.03em' }}>{value}<span style={{ fontSize: size * 0.12, color: 'var(--muted-foreground)' }}>{unit}</span></span>
        {label && <span className="muted" style={{ fontSize: 10.5, marginTop: 1 }}>{label}</span>}
      </div>
    </div>
  );
}

/* KPI stat card */
function KPI({ label, value, unit, delta, deltaDir, spark, sparkColor = 'var(--primary)', icon, sub }) {
  const up = deltaDir === 'up';
  const dColor = deltaDir ? (up ? 'var(--positive)' : 'var(--sev-fault)') : 'var(--muted-foreground)';
  return (
    <div className="card" style={{ padding: '15px 16px', display: 'flex', flexDirection: 'column', gap: 10, minWidth: 0 }}>
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: 8 }}>
        <span className="muted" style={{ fontSize: 12.5, fontWeight: 500, display: 'flex', alignItems: 'center', gap: 7 }}>
          {icon && <Icon name={icon} size={14} />}{label}
        </span>
        {delta != null && <span className="badge" style={{ background: 'transparent', padding: 0, color: dColor, fontWeight: 600 }}>
          <Icon name={up ? 'trending-up' : 'trending-down'} size={13} />{delta}
        </span>}
      </div>
      <div style={{ display: 'flex', alignItems: 'flex-end', justifyContent: 'space-between', gap: 10 }}>
        <div style={{ display: 'flex', alignItems: 'baseline', gap: 4, minWidth: 0 }}>
          <span className="tabular" style={{ fontSize: 27, fontWeight: 650, letterSpacing: '-0.035em', lineHeight: 1 }}>{value}</span>
          {unit && <span className="muted" style={{ fontSize: 13, fontWeight: 500 }}>{unit}</span>}
        </div>
        {spark && <div style={{ flex: 'none' }}><Sparkline data={spark} w={84} h={30} color={sparkColor} /></div>}
      </div>
      {sub && <div className="muted" style={{ fontSize: 11.5 }}>{sub}</div>}
    </div>
  );
}

/* legend row */
function Legend({ items }) {
  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 9 }}>
      {items.map((it, i) => (
        <div key={i} style={{ display: 'flex', alignItems: 'center', gap: 9, fontSize: 12.5 }}>
          <span style={{ width: 9, height: 9, borderRadius: 3, background: it.color, flex: 'none' }} />
          <span style={{ flex: 1, color: 'var(--muted-foreground)' }}>{it.label}</span>
          <span className="tabular" style={{ fontWeight: 600 }}>{it.kw}<span className="muted" style={{ fontWeight: 400 }}> kW</span></span>
        </div>
      ))}
    </div>
  );
}

/* section header used across surfaces */
function SectionHead({ title, sub, children }) {
  return (
    <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: 12, marginBottom: 14 }}>
      <div>
        <h3 style={{ fontSize: 14.5, fontWeight: 600 }}>{title}</h3>
        {sub && <p className="muted" style={{ fontSize: 12.5, marginTop: 2 }}>{sub}</p>}
      </div>
      {children && <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>{children}</div>}
    </div>
  );
}

/* image-placeholder for floor maps etc */
function Placeholder({ label, h = 160, icon = 'map' }) {
  return (
    <div style={{
      height: h, borderRadius: 'var(--radius)', display: 'flex', flexDirection: 'column', gap: 8,
      alignItems: 'center', justifyContent: 'center', color: 'var(--muted-foreground)',
      border: '1px solid var(--border)',
      background: 'repeating-linear-gradient(135deg, var(--subtle), var(--subtle) 10px, transparent 10px, transparent 20px)',
    }}>
      <Icon name={icon} size={22} />
      <span className="mono" style={{ fontSize: 11 }}>{label}</span>
    </div>
  );
}

Object.assign(window, { Sparkline, AreaChart, BarChart, Donut, Gauge, KPI, Legend, SectionHead, Placeholder, pathFrom });
