/* RUBIX — app chrome: sidebar, topbar, switchers */

const ACCENTS = [
  { id: 'blue', label: 'Blue', sw: 'oklch(0.62 0.19 256)' },
  { id: 'violet', label: 'Violet', sw: 'oklch(0.62 0.21 295)' },
  { id: 'emerald', label: 'Emerald', sw: 'oklch(0.64 0.15 162)' },
  { id: 'cyan', label: 'Cyan', sw: 'oklch(0.68 0.13 210)' },
  { id: 'orange', label: 'Orange', sw: 'oklch(0.7 0.18 48)' },
  { id: 'zinc', label: 'Mono', sw: 'oklch(0.6 0.01 264)' },
];

function Logo({ collapsed }) {
  return (
    <div style={{ display: 'flex', alignItems: 'center', gap: 10, minWidth: 0 }}>
      <div style={{
        width: 30, height: 30, borderRadius: 9, flex: 'none', display: 'grid', placeItems: 'center',
        background: 'linear-gradient(150deg, var(--primary), color-mix(in oklch, var(--primary) 50%, #000))',
        boxShadow: 'var(--shadow-sm), inset 0 0 0 1px rgba(255,255,255,0.14)', color: '#fff',
      }}>
        <Icon name="logo" size={17} stroke={2.2} />
      </div>
      {!collapsed && <div style={{ minWidth: 0 }}>
        <div style={{ fontSize: 14.5, fontWeight: 650, letterSpacing: '-0.02em', lineHeight: 1 }}>Rubix</div>
        <div className="muted" style={{ fontSize: 10.5, marginTop: 2, letterSpacing: '0.02em' }}>Building Intelligence</div>
      </div>}
    </div>
  );
}

function SiteSwitcher({ activeSite, setSite, collapsed }) {
  return (
    <Dropdown width={264} trigger={
      <button className="focusable" style={{
        display: 'flex', alignItems: 'center', gap: 9, width: '100%', padding: collapsed ? 7 : '8px 9px',
        borderRadius: 'var(--radius)', border: '1px solid var(--border)', background: 'var(--card)',
        boxShadow: 'var(--shadow-sm)', justifyContent: collapsed ? 'center' : 'flex-start',
      }}>
        <span style={{ width: 26, height: 26, borderRadius: 7, flex: 'none', display: 'grid', placeItems: 'center', background: 'var(--accent-bg)', color: 'var(--primary)' }}>
          <Icon name="building" size={15} />
        </span>
        {!collapsed && <><div style={{ flex: 1, minWidth: 0, textAlign: 'left' }}>
          <div style={{ fontSize: 13, fontWeight: 600, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{activeSite.name}</div>
          <div className="muted" style={{ fontSize: 11, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{activeSite.org} · {activeSite.city}</div>
        </div>
        <Icon name="chevrons-up-down" size={15} style={{ color: 'var(--muted-foreground)' }} /></>}
      </button>
    }>
      <MenuLabel>Switch site</MenuLabel>
      {SITES.map(s => (
        <MenuItem key={s.id} onClick={() => setSite(s)} active={s.id === activeSite.id}
          icon={s.online ? 'building' : 'building'}>
          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: 8 }}>
            <span>{s.name}</span>
            <span style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
              {s.alarms > 0 && <span className="badge bg-sev-fault" style={{ height: 16, padding: '0 5px', fontSize: 10 }}>{s.alarms}</span>}
              <span style={{ width: 7, height: 7, borderRadius: 99, background: s.online ? 'var(--positive)' : 'var(--muted-foreground)' }} />
            </span>
          </div>
        </MenuItem>
      ))}
      <MenuSep />
      <MenuItem icon="plus">Add a site…</MenuItem>
    </Dropdown>
  );
}

function NavBtn({ item, active, collapsed, onClick, badgeCount }) {
  return (
    <button onClick={onClick} className="focusable tt" data-tip={collapsed ? item.label : null} style={{
      display: 'flex', alignItems: 'center', gap: 10, width: '100%', position: 'relative',
      padding: collapsed ? '9px' : '8px 10px', borderRadius: 8, justifyContent: collapsed ? 'center' : 'flex-start',
      fontSize: 13.5, fontWeight: active ? 600 : 500,
      color: active ? 'var(--foreground)' : 'var(--muted-foreground)',
      background: active ? 'var(--accent-bg)' : 'transparent', transition: 'background .14s, color .14s',
    }}
      onMouseEnter={e => { if (!active) { e.currentTarget.style.background = 'var(--accent-bg)'; e.currentTarget.style.color = 'var(--foreground)'; } }}
      onMouseLeave={e => { if (!active) { e.currentTarget.style.background = 'transparent'; e.currentTarget.style.color = 'var(--muted-foreground)'; } }}>
      {active && <span style={{ position: 'absolute', left: collapsed ? 4 : 0, top: '50%', transform: 'translateY(-50%)', width: 3, height: 18, borderRadius: 99, background: 'var(--primary)' }} />}
      <Icon name={item.icon} size={17} stroke={active ? 2.2 : 2} style={{ color: active ? 'var(--primary)' : 'inherit' }} />
      {!collapsed && <span style={{ flex: 1, textAlign: 'left' }}>{item.label}</span>}
      {!collapsed && badgeCount > 0 && <span className="badge bg-sev-fault" style={{ height: 18, padding: '0 6px', fontSize: 11 }}>{badgeCount}</span>}
      {collapsed && badgeCount > 0 && <span style={{ position: 'absolute', top: 6, right: 6, width: 7, height: 7, borderRadius: 99, background: 'var(--sev-fault)' }} />}
    </button>
  );
}

function Sidebar({ view, setView, collapsed, setCollapsed, activeSite, setSite, openSparks }) {
  const sparkCount = SPARKS.filter(s => !s.ack).length;
  return (
    <aside style={{
      width: collapsed ? 64 : 248, flex: 'none', height: '100%', display: 'flex', flexDirection: 'column',
      background: 'var(--sidebar)', borderRight: '1px solid var(--sidebar-border)', transition: 'width .18s cubic-bezier(0.3,0.7,0.3,1)',
    }}>
      <div style={{ padding: collapsed ? '14px 12px' : '14px 16px', display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
        <Logo collapsed={collapsed} />
      </div>
      <div style={{ padding: collapsed ? '0 12px 10px' : '0 14px 12px' }}>
        <SiteSwitcher activeSite={activeSite} setSite={setSite} collapsed={collapsed} />
      </div>

      <nav className="scroll" style={{ flex: 1, overflowY: 'auto', padding: collapsed ? '4px 12px' : '4px 12px', display: 'flex', flexDirection: 'column', gap: 2 }}>
        {NAV.map(it => <NavBtn key={it.id} item={it} active={view === it.id} collapsed={collapsed}
          onClick={() => setView(it.id)} badgeCount={it.badge === 'sparks' ? sparkCount : 0} />)}
        <div style={{ height: 1, background: 'var(--sidebar-border)', margin: collapsed ? '10px 4px' : '10px 6px' }} />
        {!collapsed && <div className="eyebrow" style={{ padding: '2px 10px 4px', fontSize: 10 }}>Analyze</div>}
        {NAV2.map(it => <NavBtn key={it.id} item={it} active={view === it.id} collapsed={collapsed} onClick={() => setView(it.id)} />)}
      </nav>

      <div style={{ padding: collapsed ? '8px 12px' : '8px 14px', display: 'flex', flexDirection: 'column', gap: 6 }}>
        <button onClick={() => setCollapsed(c => !c)} className="focusable" style={{
          display: 'flex', alignItems: 'center', gap: 9, padding: '7px 9px', borderRadius: 8, color: 'var(--muted-foreground)', fontSize: 12.5, justifyContent: collapsed ? 'center' : 'flex-start',
        }} onMouseEnter={e => e.currentTarget.style.background = 'var(--accent-bg)'} onMouseLeave={e => e.currentTarget.style.background = 'transparent'}>
          <Icon name="panel-left" size={16} />{!collapsed && 'Collapse'}
        </button>
        <UserNav collapsed={collapsed} />
      </div>
    </aside>
  );
}

function UserNav({ collapsed }) {
  return (
    <Dropdown width={250} side="top" trigger={
      <button className="focusable" style={{
        display: 'flex', alignItems: 'center', gap: 10, width: '100%', padding: collapsed ? 6 : '7px 8px',
        borderRadius: 'var(--radius)', justifyContent: collapsed ? 'center' : 'flex-start',
      }} onMouseEnter={e => e.currentTarget.style.background = 'var(--accent-bg)'} onMouseLeave={e => e.currentTarget.style.background = 'transparent'}>
        <Avatar name="Dana Okafor" size={collapsed ? 30 : 30} />
        {!collapsed && <><div style={{ flex: 1, minWidth: 0, textAlign: 'left' }}>
          <div style={{ fontSize: 12.5, fontWeight: 600, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>Dana Okafor</div>
          <div className="muted" style={{ fontSize: 11, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>Lead Operator</div>
        </div>
        <Icon name="chevrons-up-down" size={15} style={{ color: 'var(--muted-foreground)' }} /></>}
      </button>
    }>
      <div style={{ display: 'flex', alignItems: 'center', gap: 10, padding: '8px 8px 10px' }}>
        <Avatar name="Dana Okafor" size={36} />
        <div style={{ minWidth: 0 }}>
          <div style={{ fontSize: 13, fontWeight: 600 }}>Dana Okafor</div>
          <div className="muted" style={{ fontSize: 11.5 }}>dana@acme.io</div>
        </div>
      </div>
      <MenuSep />
      <MenuItem icon="user">Profile & preferences</MenuItem>
      <MenuItem icon="users">Team & roles</MenuItem>
      <MenuItem icon="shield-check">Tokens & service accounts</MenuItem>
      <MenuItem icon="credit-card">Billing</MenuItem>
      <MenuSep />
      <MenuItem icon="book-open">Docs</MenuItem>
      <MenuItem icon="life-buoy">Support</MenuItem>
      <MenuSep />
      <MenuItem icon="log-out" danger>Sign out</MenuItem>
    </Dropdown>
  );
}

function ThemeToggle({ theme, setTheme }) {
  return (
    <button className="btn btn-ghost btn-icon btn-sm tt" data-tip={theme === 'dark' ? 'Light mode' : 'Dark mode'} onClick={() => setTheme(theme === 'dark' ? 'light' : 'dark')}>
      <Icon name={theme === 'dark' ? 'sun' : 'moon'} size={17} />
    </button>
  );
}

function ColorSwitcher({ accent, setAccent }) {
  return (
    <Dropdown align="end" width={210} trigger={
      <button className="btn btn-ghost btn-icon btn-sm tt" data-tip="Accent color">
        <Icon name="palette" size={17} />
      </button>
    }>
      <MenuLabel>Accent color</MenuLabel>
      <div data-keep-open style={{ display: 'grid', gridTemplateColumns: 'repeat(3,1fr)', gap: 6, padding: '4px 6px 8px' }}>
        {ACCENTS.map(a => (
          <button key={a.id} onClick={() => setAccent(a.id)} style={{
            display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 5, padding: '9px 4px', borderRadius: 8,
            border: '1px solid ' + (accent === a.id ? 'var(--primary)' : 'var(--border)'),
            background: accent === a.id ? 'var(--accent-bg)' : 'transparent',
          }}>
            <span style={{ width: 22, height: 22, borderRadius: 99, background: a.sw, boxShadow: 'inset 0 0 0 1px rgba(255,255,255,0.15)', display: 'grid', placeItems: 'center' }}>
              {accent === a.id && <Icon name="check" size={13} style={{ color: '#fff' }} stroke={3} />}
            </span>
            <span style={{ fontSize: 10.5, fontWeight: 500, color: accent === a.id ? 'var(--foreground)' : 'var(--muted-foreground)' }}>{a.label}</span>
          </button>
        ))}
      </div>
    </Dropdown>
  );
}

function Topbar({ view, theme, setTheme, accent, setAccent, openCommand, setView, activeSite }) {
  const titles = {
    dashboard: ['Dashboard', activeSite.name + ' · live overview'],
    builder: ['Dashboard Builder', 'Compose and bind widgets'],
    sparks: ['Sparks', 'Rule findings across your portfolio'],
    points: ['Points & Equipment', activeSite.name + ' · ' + activeSite.points.toLocaleString() + ' points'],
    flows: ['Flow Boards', 'reflow control & analytics graphs'],
    history: ['History & SQL', 'DataFusion query surface'],
    runs: ['Agent Runs', 'awaken activity & approvals'],
  };
  const [t, sub] = titles[view] || ['Rubix', ''];
  const sparkCount = SPARKS.filter(s => !s.ack).length;
  return (
    <header style={{
      height: 56, flex: 'none', display: 'flex', alignItems: 'center', gap: 14, padding: '0 18px',
      borderBottom: '1px solid var(--border)', background: 'color-mix(in oklch, var(--background) 80%, transparent)',
      backdropFilter: 'blur(8px)', position: 'relative', zIndex: 20,
    }}>
      <div style={{ minWidth: 0, flex: 1 }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 9 }}>
          <h2 style={{ fontSize: 15.5, fontWeight: 650, letterSpacing: '-0.02em' }}>{t}</h2>
          <span className="live-dot" />
        </div>
        <div className="muted" style={{ fontSize: 11.5, marginTop: 1, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{sub}</div>
      </div>

      <button onClick={openCommand} className="focusable" style={{
        display: 'flex', alignItems: 'center', gap: 10, height: 36, padding: '0 10px 0 12px', minWidth: 280,
        borderRadius: 'var(--radius)', border: '1px solid var(--border)', background: 'var(--card)', boxShadow: 'var(--shadow-sm)', color: 'var(--muted-foreground)',
      }}>
        <Icon name="sparkles" size={16} style={{ color: 'var(--primary)' }} />
        <span style={{ flex: 1, textAlign: 'left', fontSize: 13 }}>Ask awaken or search…</span>
        <span style={{ display: 'flex', gap: 3 }}><span className="kbd">⌘</span><span className="kbd">K</span></span>
      </button>

      <div style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
        <button onClick={() => setView('sparks')} className="btn btn-ghost btn-icon btn-sm tt" data-tip="Sparks" style={{ position: 'relative' }}>
          <Icon name="bell" size={17} />
          {sparkCount > 0 && <span style={{ position: 'absolute', top: 5, right: 5, minWidth: 14, height: 14, padding: '0 3px', borderRadius: 99, background: 'var(--sev-fault)', color: '#fff', fontSize: 9.5, fontWeight: 700, display: 'grid', placeItems: 'center', boxShadow: '0 0 0 2px var(--background)' }}>{sparkCount}</span>}
        </button>
        <ColorSwitcher accent={accent} setAccent={setAccent} />
        <ThemeToggle theme={theme} setTheme={setTheme} />
      </div>
    </header>
  );
}

Object.assign(window, { Sidebar, Topbar, UserNav, SiteSwitcher, ColorSwitcher, ThemeToggle, Logo, ACCENTS });
