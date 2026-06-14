/* RUBIX — app root: state, routing, layout */

function useLocalState(key, init) {
  const [v, setV] = React.useState(() => {
    try { const s = localStorage.getItem(key); return s == null ? init : JSON.parse(s); } catch { return init; }
  });
  React.useEffect(() => { try { localStorage.setItem(key, JSON.stringify(v)); } catch {} }, [key, v]);
  return [v, setV];
}

function App() {
  const [view, setView] = useLocalState('rubix.view', 'dashboard');
  const [theme, setTheme] = useLocalState('rubix.theme', 'dark');
  const [accent, setAccent] = useLocalState('rubix.accent', 'blue');
  const [collapsed, setCollapsed] = useLocalState('rubix.collapsed', false);
  const [siteId, setSiteId] = useLocalState('rubix.site', 's1');
  const [cmdOpen, setCmdOpen] = React.useState(false);
  const activeSite = SITES.find(s => s.id === siteId) || SITES[0];
  const setSite = (s) => setSiteId(s.id);

  React.useEffect(() => {
    const r = document.documentElement;
    r.classList.toggle('dark', theme === 'dark');
    r.setAttribute('data-accent', accent);
  }, [theme, accent]);

  React.useEffect(() => {
    const h = (e) => {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'k') { e.preventDefault(); setCmdOpen(o => !o); }
    };
    window.addEventListener('keydown', h);
    return () => window.removeEventListener('keydown', h);
  }, []);

  const views = {
    dashboard: <Dashboard activeSite={activeSite} setView={setView} />,
    builder: <Builder activeSite={activeSite} />,
    sparks: <Sparks setView={setView} openAgent={() => setCmdOpen(true)} />,
    points: <Points activeSite={activeSite} />,
    flows: <Flows />,
    history: <SimplePage icon="database" title="History & SQL" text="DataFusion query surface — points_cur, his, sparks tables federated over Postgres + Parquet." />,
    runs: <SimplePage icon="sparkles" title="Agent Runs" text="awaken run records, tool traces, and approval history live here." />,
  };

  return (
    <div style={{ display: 'flex', height: '100vh', width: '100vw', overflow: 'hidden' }}>
      <Sidebar view={view} setView={setView} collapsed={collapsed} setCollapsed={setCollapsed} activeSite={activeSite} setSite={setSite} />
      <div style={{ flex: 1, display: 'flex', flexDirection: 'column', minWidth: 0 }}>
        <Topbar view={view} theme={theme} setTheme={setTheme} accent={accent} setAccent={setAccent}
          openCommand={() => setCmdOpen(true)} setView={setView} activeSite={activeSite} />
        <main style={{ flex: 1, minHeight: 0, display: 'flex', flexDirection: 'column', background: 'var(--background)' }}>
          {views[view] || views.dashboard}
        </main>
      </div>
      <CommandPalette open={cmdOpen} onClose={() => setCmdOpen(false)} setView={setView} />
    </div>
  );
}

function SimplePage({ icon, title, text }) {
  return (
    <div style={{ flex: 1, display: 'grid', placeItems: 'center', padding: 40 }}>
      <div style={{ textAlign: 'center', maxWidth: 380 }}>
        <div style={{ width: 56, height: 56, borderRadius: 14, margin: '0 auto 16px', display: 'grid', placeItems: 'center', background: 'var(--accent-bg)', color: 'var(--primary)' }}>
          <Icon name={icon} size={26} />
        </div>
        <h3 style={{ fontSize: 18, marginBottom: 8 }}>{title}</h3>
        <p className="muted" style={{ fontSize: 13.5, lineHeight: 1.6 }}>{text}</p>
      </div>
    </div>
  );
}

ReactDOM.createRoot(document.getElementById('root')).render(<App />);
