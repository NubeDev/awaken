/* RUBIX — primitives: icons + shadcn-flavoured atoms */

/* ---------- lucide-style icon set ---------- */
const ICONS = {
  // nav
  gauge: 'M12 14l4-4M3.34 19a10 10 0 1 1 17.32 0',
  'layout-dashboard': 'M3 3h7v9H3zM14 3h7v5h-7zM14 12h7v9h-7zM3 16h7v5H3z',
  blocks: 'M10 4H4a1 1 0 0 0-1 1v6a1 1 0 0 0 1 1h6a1 1 0 0 0 1-1V5a1 1 0 0 0-1-1zM15 3h5a1 1 0 0 1 1 1v5M21 14v5a1 1 0 0 1-1 1h-5M14 14h6v6h-6z',
  zap: 'M4 14h7l-1 8 10-12h-7l1-8z',
  workflow: 'M3 3h6v6H3zM15 15h6v6h-6zM9 6h6a2 2 0 0 1 2 2v7M6 9v6a2 2 0 0 0 2 2h7',
  'git-branch': 'M6 3v12M18 9a3 3 0 1 0 0-6 3 3 0 0 0 0 6zM6 21a3 3 0 1 0 0-6 3 3 0 0 0 0 6zM15 6a9 9 0 0 1-9 9',
  binary: 'M6 4h4v6H6zM14 14h4v6h-4zM6 20h4M16 4h2v6M6 14h2M14 10h4',
  network: 'M9 2h6v6H9zM2 16h6v6H2zM16 16h6v6h-6zM5 16v-3a1 1 0 0 1 1-1h12a1 1 0 0 1 1 1v3M12 12V8',
  cpu: 'M9 9h6v6H9zM4 9h2M4 15h2M18 9h2M18 15h2M9 4v2M15 4v2M9 18v2M15 18v2M6 6h12a1 1 0 0 1 1 1v10a1 1 0 0 1-1 1H6a1 1 0 0 1-1-1V7a1 1 0 0 1 1-1z',
  activity: 'M22 12h-4l-3 9L9 3l-3 9H2',
  // sparks / alerts
  'triangle-alert': 'M10.3 3.9 1.8 18a2 2 0 0 0 1.7 3h17a2 2 0 0 0 1.7-3L13.7 3.9a2 2 0 0 0-3.4 0zM12 9v4M12 17h.01',
  'circle-alert': 'M12 22a10 10 0 1 0 0-20 10 10 0 0 0 0 20zM12 8v4M12 16h.01',
  info: 'M12 22a10 10 0 1 0 0-20 10 10 0 0 0 0 20zM12 16v-4M12 8h.01',
  bell: 'M10.3 21a1.94 1.94 0 0 0 3.4 0M3.3 16a1 1 0 0 0 .7 1.7h16a1 1 0 0 0 .7-1.7C19.7 14.6 18 13 18 8A6 6 0 0 0 6 8c0 5-1.7 6.6-2.7 8z',
  flame: 'M12 22c4.4 0 8-3.5 8-7.8 0-2.6-1.6-5.1-3-6.7-.4 1.3-1.5 2-2.5 2 .8-2.4-.3-5-2-6.5-.3 2.2-1.6 3.5-3 4.7C7 9 6 11 6 13.5 6 18 8.6 22 12 22z',
  // values / actions
  check: 'M20 6 9 17l-5-5',
  'check-check': 'M2 12l5 5L18 6M22 6l-7.5 7.5',
  x: 'M18 6 6 18M6 6l12 12',
  plus: 'M5 12h14M12 5v14',
  minus: 'M5 12h14',
  search: 'M21 21l-4.3-4.3M11 19a8 8 0 1 0 0-16 8 8 0 0 0 0 16z',
  'sliders-horizontal': 'M21 4h-7M10 4H3M21 12h-9M8 12H3M21 20h-5M12 20H3M14 2v4M8 10v4M16 18v4',
  filter: 'M22 3H2l8 9.5V19l4 2v-8.5z',
  'chevron-down': 'M6 9l6 6 6-6',
  'chevron-right': 'M9 18l6-6-6-6',
  'chevron-left': 'M15 18l-6-6 6-6',
  'chevrons-up-down': 'M7 15l5 5 5-5M7 9l5-5 5 5',
  'arrow-up-right': 'M7 17 17 7M7 7h10v10',
  'arrow-right': 'M5 12h14M13 6l6 6-6 6',
  'trending-up': 'M22 7 13.5 15.5l-5-5L2 17M16 7h6v6',
  'trending-down': 'M22 17 13.5 8.5l-5 5L2 7M16 17h6v-6',
  // chrome
  command: 'M15 6a3 3 0 1 0 3 3H6a3 3 0 1 0 3-3v12a3 3 0 1 0-3-3h12a3 3 0 1 0-3 3z',
  sun: 'M12 16a4 4 0 1 0 0-8 4 4 0 0 0 0 8zM12 2v2M12 20v2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M2 12h2M20 12h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4',
  moon: 'M12 3a6 6 0 0 0 9 9 9 9 0 1 1-9-9z',
  palette: 'M12 22a10 10 0 1 1 0-20 8 8 0 0 1 8 8 4 4 0 0 1-4 4h-2a2 2 0 0 0-1.5 3.3A2 2 0 0 1 12 22zM7.5 11a1 1 0 1 0 0-2 1 1 0 0 0 0 2zM12 7.5a1 1 0 1 0 0-2 1 1 0 0 0 0 2zM16.5 11a1 1 0 1 0 0-2 1 1 0 0 0 0 2z',
  settings: 'M12.2 2h-.4a2 2 0 0 0-2 2 1.7 1.7 0 0 1-2.6 1.5 2 2 0 0 0-2.7.7l-.2.4a2 2 0 0 0 .7 2.7 1.7 1.7 0 0 1 0 3 2 2 0 0 0-.7 2.7l.2.4a2 2 0 0 0 2.7.7A1.7 1.7 0 0 1 9.8 20a2 2 0 0 0 2 2h.4a2 2 0 0 0 2-2 1.7 1.7 0 0 1 2.6-1.5 2 2 0 0 0 2.7-.7l.2-.4a2 2 0 0 0-.7-2.7 1.7 1.7 0 0 1 0-3 2 2 0 0 0 .7-2.7l-.2-.4a2 2 0 0 0-2.7-.7A1.7 1.7 0 0 1 14.2 4a2 2 0 0 0-2-2zM12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6z',
  'panel-left': 'M3 3h18v18H3zM9 3v18',
  'log-out': 'M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4M16 17l5-5-5-5M21 12H9',
  'user': 'M19 21v-2a4 4 0 0 0-4-4H9a4 4 0 0 0-4 4v2M12 11a4 4 0 1 0 0-8 4 4 0 0 0 0 8z',
  'users': 'M16 21v-2a4 4 0 0 0-3-3.9M2 21v-2a4 4 0 0 1 3-3.9M9 11a4 4 0 1 0 0-8 4 4 0 0 0 0 8zM16 3.1a4 4 0 0 1 0 7.8',
  'credit-card': 'M3 5h18a1 1 0 0 1 1 1v12a1 1 0 0 1-1 1H3a1 1 0 0 1-1-1V6a1 1 0 0 1 1-1zM2 10h20',
  'life-buoy': 'M12 22a10 10 0 1 0 0-20 10 10 0 0 0 0 20zM12 16a4 4 0 1 0 0-8 4 4 0 0 0 0 8zM4.9 4.9l4.2 4.2M14.9 14.9l4.2 4.2M14.9 9.1l4.2-4.2M4.9 19.1l4.2-4.2',
  'book-open': 'M12 7v14M3 18a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1h5a4 4 0 0 1 4 4 4 4 0 0 1 4-4h5a1 1 0 0 1 1 1v13a1 1 0 0 1-1 1h-6a3 3 0 0 0-3 3 3 3 0 0 0-3-3z',
  // domain
  building: 'M6 22V4a1 1 0 0 1 1-1h10a1 1 0 0 1 1 1v18M2 22h20M9 7h.01M14 7h.01M9 11h.01M14 11h.01M9 15h.01M14 15h.01',
  'thermometer': 'M14 14.8V4a2 2 0 1 0-4 0v10.8a4 4 0 1 0 4 0z',
  fan: 'M12 12a3 3 0 1 0 0 0zM12 12c-2.8 0-5-1.2-5-4 0-1.6 1.8-3 3-3 2.8 0 2 4 2 7zM12 12c0 2.8 1.2 5 4 5 1.6 0 3-1.8 3-3 0-2.8-4-2-7-2zM12 12c-2.8 0-5 1.2-5 4 0-1.6 1.4-3 3-3',
  droplet: 'M12 22a7 7 0 0 0 7-7c0-2-1-4-3-6l-4-5-4 5c-2 2-3 4-3 6a7 7 0 0 0 7 7z',
  wind: 'M12.8 19.6A2 2 0 1 0 14 16H2M17.5 8a2.5 2.5 0 1 1 1.8 4.2H2M9.8 4.4A2 2 0 1 1 11 8H2',
  plug: 'M12 22v-5M9 8V2M15 8V2M18 8H6a4 4 0 0 0 4 4h4a4 4 0 0 0 4-4z',
  power: 'M12 2v10M18.4 6.6a9 9 0 1 1-12.8 0',
  'circle-dot': 'M12 22a10 10 0 1 0 0-20 10 10 0 0 0 0 20zM12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6z',
  'circle': 'M12 22a10 10 0 1 0 0-20 10 10 0 0 0 0 20z',
  dot: 'M12 13a1 1 0 1 0 0-2 1 1 0 0 0 0 2z',
  table: 'M3 3h18v18H3zM3 9h18M3 15h18M9 3v18',
  'line-chart': 'M3 3v18h18M7 14l3-4 3 2 5-6',
  'bar-chart': 'M3 3v18h18M8 17v-5M13 17V8M18 17v-9',
  'pie-chart': 'M21 12A9 9 0 1 1 9 3.5M12 3a9 9 0 0 1 9 9h-9z',
  map: 'M9 4 3 6v14l6-2 6 2 6-2V4l-6 2-6-2zM9 4v14M15 6v14',
  list: 'M8 6h13M8 12h13M8 18h13M3 6h.01M3 12h.01M3 18h.01',
  send: 'M14.5 9.5 21 3M21 3l-6.5 18a.5.5 0 0 1-.9 0L10 14l-7-3.6a.5.5 0 0 1 0-.9z',
  sparkles: 'M9.9 4.2 11 8l3.8 1.1L11 10.2 9.9 14 8.8 10.2 5 9.1 8.8 8zM18 4v3M19.5 5.5h-3M18 17v3M19.5 18.5h-3',
  'wand': 'M15 4V2M15 16v-2M8 9h2M20 9h2M17.8 11.8 19 13M15 9h0M17.8 6.2 19 5M3 21l9-9M12.2 6.2 11 5',
  copy: 'M9 9h11a1 1 0 0 1 1 1v11a1 1 0 0 1-1 1H9a1 1 0 0 1-1-1V10a1 1 0 0 1 1-1zM5 15H4a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1h10a1 1 0 0 1 1 1v1',
  trash: 'M3 6h18M8 6V4a1 1 0 0 1 1-1h6a1 1 0 0 1 1 1v2M19 6l-1 14a1 1 0 0 1-1 1H7a1 1 0 0 1-1-1L5 6M10 11v6M14 11v6',
  'grip-vertical': 'M9 5h.01M9 12h.01M9 19h.01M15 5h.01M15 12h.01M15 19h.01',
  'grip': 'M5 9h.01M5 15h.01M12 9h.01M12 15h.01M19 9h.01M19 15h.01',
  pin: 'M12 17v5M9 10.8V4h6v6.8a2 2 0 0 0 .6 1.4l1.9 1.9a1 1 0 0 1-.7 1.7H7.2a1 1 0 0 1-.7-1.7l1.9-1.9a2 2 0 0 0 .6-1.4z',
  maximize: 'M8 3H5a2 2 0 0 0-2 2v3M16 3h3a2 2 0 0 1 2 2v3M8 21H5a2 2 0 0 1-2-2v-3M16 21h3a2 2 0 0 0 2-2v-3',
  'refresh': 'M3 12a9 9 0 0 1 15-6.7L21 8M21 3v5h-5M21 12a9 9 0 0 1-15 6.7L3 16M3 21v-5h5',
  clock: 'M12 22a10 10 0 1 0 0-20 10 10 0 0 0 0 20zM12 6v6l4 2',
  history: 'M3 3v5h5M3.05 13A9 9 0 1 0 6 5.3L3 8M12 7v5l4 2',
  calendar: 'M8 2v4M16 2v4M3 7h18M3 10a1 1 0 0 1 1-1h16a1 1 0 0 1 1 1v10a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1z',
  'shield-check': 'M20 13c0 5-3.5 7.5-7.7 9a1 1 0 0 1-.6 0C7.5 20.5 4 18 4 13V6a1 1 0 0 1 1-1c2 0 4.5-1.2 6.2-2.7a1 1 0 0 1 1.6 0C14.5 3.8 17 5 19 5a1 1 0 0 1 1 1zM9 12l2 2 4-4',
  lock: 'M5 11h14a1 1 0 0 1 1 1v8a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1v-8a1 1 0 0 1 1-1zM8 11V7a4 4 0 0 1 8 0v4',
  hand: 'M18 11V6a2 2 0 0 0-4 0M14 10V4a2 2 0 0 0-4 0v2M10 10.5V6a2 2 0 0 0-4 0v8M18 11a6 6 0 0 1-6 6c-2.5 0-3.5-1-5-2.5L4.5 12a1.8 1.8 0 0 1 2.6-2.5L9 11M18 8v3a2 2 0 1 0 4 0V8',
  'play': 'M6 4l14 8-14 8z',
  'square-pen': 'M11 4H4a1 1 0 0 0-1 1v15a1 1 0 0 0 1 1h15a1 1 0 0 0 1-1v-7M18.4 2.6a2 2 0 0 1 3 3L12 15l-4 1 1-4z',
  'eye': 'M2 12s3.5-7 10-7 10 7 10 7-3.5 7-10 7-10-7-10-7zM12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6z',
  'database': 'M12 8c5 0 8-1.3 8-3s-3-3-8-3-8 1.3-8 3 3 3 8 3zM4 5v6c0 1.7 3 3 8 3s8-1.3 8-3V5M4 11v6c0 1.7 3 3 8 3s8-1.3 8-3v-6',
  'function': 'M9 17c0 1.7-1.3 3-3 3M14 21c-1 0-2-1-2-3V6c0-2 1-3 3-3M8 12h7',
  'terminal': 'M4 17l6-5-6-5M12 19h8',
  'message-square': 'M21 15a2 2 0 0 1-2 2H8l-4 4V5a2 2 0 0 1 2-2h13a2 2 0 0 1 2 2z',
  'corner-down-left': 'M9 10l-5 5 5 5M20 4v7a4 4 0 0 1-4 4H4',
  'box': 'M21 8 12 3 3 8v8l9 5 9-5zM3 8l9 5 9-5M12 22V13',
  'folder': 'M4 4h5l2 3h9a1 1 0 0 1 1 1v11a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1V5a1 1 0 0 1 1-1z',
  'logo': 'M4 7l8-4 8 4v10l-8 4-8-4zM4 7l8 4 8-4M12 11v10',
  'star': 'M12 2.5l2.9 5.9 6.6 1-4.8 4.6 1.1 6.5L12 17.4 6.2 20.5l1.1-6.5L2.5 9.4l6.6-1z',
  'maximize2': 'M15 3h6v6M9 21H3v-6M21 3l-7 7M3 21l7-7',
  'gauge-circle': 'M15.6 8.4 12 12M2 12a10 10 0 1 0 20 0 10 10 0 0 0-20 0zM12 12h.01',
  'scan': 'M3 7V5a2 2 0 0 1 2-2h2M17 3h2a2 2 0 0 1 2 2v2M21 17v2a2 2 0 0 1-2 2h-2M7 21H5a2 2 0 0 1-2-2v-2',
  'more-horizontal': 'M12 13a1 1 0 1 0 0-2 1 1 0 0 0 0 2zM19 13a1 1 0 1 0 0-2 1 1 0 0 0 0 2zM5 13a1 1 0 1 0 0-2 1 1 0 0 0 0 2z',
  'arrow-down': 'M12 5v14M19 12l-7 7-7-7',
  'sigma': 'M18 7V4H6l6 8-6 8h12v-3',
  'route': 'M6 19a3 3 0 1 0 0-6 3 3 0 0 0 0 6zM18 11a3 3 0 1 0 0-6 3 3 0 0 0 0 6zM9 19h7a3 3 0 0 0 0-6h-4a3 3 0 0 1 0-6H8',
};

function Icon({ name, size = 16, stroke = 2, className = '', style = {}, fill = 'none' }) {
  const d = ICONS[name];
  if (!d) return null;
  const filled = name === 'logo-fill';
  return (
    <svg className={className} width={size} height={size} viewBox="0 0 24 24"
      fill={fill} stroke="currentColor" strokeWidth={stroke} strokeLinecap="round" strokeLinejoin="round"
      style={{ flex: 'none', ...style }} aria-hidden="true">
      {d.split('M').filter(Boolean).map((seg, i) => <path key={i} d={'M' + seg} />)}
    </svg>
  );
}

const cx = (...a) => a.filter(Boolean).join(' ');

/* ---------- Avatar ---------- */
function Avatar({ name = '', src, size = 28, accent }) {
  const initials = name.split(' ').map(w => w[0]).slice(0, 2).join('').toUpperCase();
  return (
    <div style={{
      width: size, height: size, borderRadius: '50%', flex: 'none',
      display: 'flex', alignItems: 'center', justifyContent: 'center',
      fontSize: size * 0.38, fontWeight: 600, color: '#fff', letterSpacing: 0,
      background: accent || 'linear-gradient(135deg, var(--primary), color-mix(in oklch, var(--primary) 55%, #7c3aed))',
      boxShadow: 'inset 0 0 0 1px rgba(255,255,255,0.12)',
    }}>{initials}</div>
  );
}

/* ---------- Dropdown ---------- */
function Dropdown({ trigger, children, align = 'start', width = 220, side = 'bottom' }) {
  const [open, setOpen] = React.useState(false);
  const ref = React.useRef(null);
  React.useEffect(() => {
    if (!open) return;
    const h = (e) => { if (ref.current && !ref.current.contains(e.target)) setOpen(false); };
    const k = (e) => { if (e.key === 'Escape') setOpen(false); };
    document.addEventListener('mousedown', h); document.addEventListener('keydown', k);
    return () => { document.removeEventListener('mousedown', h); document.removeEventListener('keydown', k); };
  }, [open]);
  const pos = side === 'top' ? { bottom: 'calc(100% + 6px)' } : { top: 'calc(100% + 6px)' };
  pos[align === 'end' ? 'right' : 'left'] = 0;
  return (
    <div ref={ref} style={{ position: 'relative' }}>
      <div onClick={() => setOpen(o => !o)}>{trigger}</div>
      {open && (
        <div className="card anim-pop scroll" onClick={(e) => {
          if (e.target.closest('[data-keep-open]')) return; setOpen(false);
        }} style={{
          position: 'absolute', ...pos, width, zIndex: 70, padding: 5,
          boxShadow: 'var(--shadow-lg)', borderColor: 'var(--border-strong)', maxHeight: 420, overflowY: 'auto',
        }}>{children}</div>
      )}
    </div>
  );
}
function MenuItem({ icon, children, onClick, danger, kbd, active }) {
  return (
    <button onClick={onClick} style={{
      display: 'flex', alignItems: 'center', gap: 9, width: '100%', textAlign: 'left',
      padding: '7px 8px', borderRadius: 7, fontSize: 13.5, fontWeight: 500,
      color: danger ? 'var(--destructive)' : 'var(--foreground)',
      background: active ? 'var(--accent-bg)' : 'transparent',
    }}
      onMouseEnter={e => e.currentTarget.style.background = 'var(--accent-bg)'}
      onMouseLeave={e => e.currentTarget.style.background = active ? 'var(--accent-bg)' : 'transparent'}>
      {icon && <Icon name={icon} size={15} style={{ color: danger ? 'var(--destructive)' : 'var(--muted-foreground)' }} />}
      <span style={{ flex: 1 }}>{children}</span>
      {kbd && <span className="kbd">{kbd}</span>}
    </button>
  );
}
const MenuLabel = ({ children }) => <div style={{ padding: '6px 8px 4px', fontSize: 11, fontWeight: 600, color: 'var(--muted-foreground)', letterSpacing: '0.02em' }}>{children}</div>;
const MenuSep = () => <div style={{ height: 1, background: 'var(--border)', margin: '5px -5px' }} />;

/* ---------- Switch ---------- */
function Switch({ checked, onChange, size = 'md' }) {
  const w = size === 'sm' ? 32 : 38, h = size === 'sm' ? 18 : 21, kn = h - 6;
  return (
    <button onClick={() => onChange(!checked)} style={{
      width: w, height: h, borderRadius: 99, flex: 'none', position: 'relative',
      background: checked ? 'var(--primary)' : 'var(--border-strong)', transition: 'background .16s',
    }}>
      <span style={{
        position: 'absolute', top: 3, left: checked ? w - kn - 3 : 3, width: kn, height: kn, borderRadius: 99,
        background: '#fff', transition: 'left .16s cubic-bezier(0.2,0.7,0.3,1)', boxShadow: '0 1px 2px rgba(0,0,0,0.3)',
      }} />
    </button>
  );
}

/* ---------- Segmented ---------- */
function Segmented({ options, value, onChange, size = 'md' }) {
  return (
    <div style={{ display: 'inline-flex', padding: 3, gap: 2, background: 'var(--muted)', borderRadius: 'calc(var(--radius) - 1px)' }}>
      {options.map(o => {
        const v = o.value ?? o, label = o.label ?? o;
        const active = v === value;
        return (
          <button key={v} onClick={() => onChange(v)} style={{
            display: 'inline-flex', alignItems: 'center', gap: 6,
            height: size === 'sm' ? 24 : 28, padding: '0 10px', borderRadius: 'calc(var(--radius) - 4px)',
            fontSize: 12.5, fontWeight: 500, transition: 'all .14s',
            background: active ? 'var(--card)' : 'transparent',
            color: active ? 'var(--foreground)' : 'var(--muted-foreground)',
            boxShadow: active ? 'var(--shadow-sm)' : 'none',
          }}>
            {o.icon && <Icon name={o.icon} size={14} />}{label}
          </button>
        );
      })}
    </div>
  );
}

/* ---------- Tabs (underline) ---------- */
function Tabs({ tabs, value, onChange }) {
  return (
    <div style={{ display: 'flex', gap: 2, borderBottom: '1px solid var(--border)' }}>
      {tabs.map(t => {
        const v = t.value ?? t, label = t.label ?? t, active = v === value;
        return (
          <button key={v} onClick={() => onChange(v)} style={{
            display: 'inline-flex', alignItems: 'center', gap: 7, padding: '9px 12px', marginBottom: -1,
            fontSize: 13.5, fontWeight: 500, position: 'relative',
            color: active ? 'var(--foreground)' : 'var(--muted-foreground)',
            borderBottom: active ? '2px solid var(--primary)' : '2px solid transparent',
          }}>
            {t.icon && <Icon name={t.icon} size={15} />}{label}
            {t.count != null && <span className="badge badge-muted" style={{ height: 17, padding: '0 5px', fontSize: 10.5 }}>{t.count}</span>}
          </button>
        );
      })}
    </div>
  );
}

Object.assign(window, { Icon, ICONS, cx, Avatar, Dropdown, MenuItem, MenuLabel, MenuSep, Switch, Segmented, Tabs });
