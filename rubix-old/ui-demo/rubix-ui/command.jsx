/* RUBIX — ⌘K command palette + awaken AI agent */

function CommandPalette({ open, onClose, setView, runAgentSeed }) {
  const [q, setQ] = React.useState('');
  const [mode, setMode] = React.useState('command'); // command | agent
  const inputRef = React.useRef(null);
  const [activeIdx, setActiveIdx] = React.useState(0);

  React.useEffect(() => {
    if (open) { setMode('command'); setQ(''); setActiveIdx(0); setTimeout(() => inputRef.current?.focus(), 40); }
  }, [open]);

  const commands = React.useMemo(() => ([
    { group: 'Navigate', items: [
      { icon: 'gauge', label: 'Go to Dashboard', kbd: 'G D', act: () => { setView('dashboard'); onClose(); } },
      { icon: 'layout-dashboard', label: 'Open Dashboard Builder', act: () => { setView('builder'); onClose(); } },
      { icon: 'zap', label: 'View Sparks', kbd: 'G S', act: () => { setView('sparks'); onClose(); } },
      { icon: 'network', label: 'Browse Points & Equipment', act: () => { setView('points'); onClose(); } },
      { icon: 'workflow', label: 'Open Flow Boards', act: () => { setView('flows'); onClose(); } },
    ]},
    { group: 'Actions', items: [
      { icon: 'plus', label: 'Create dashboard', act: () => { setView('builder'); onClose(); } },
      { icon: 'square-pen', label: 'New flow board', act: () => { setView('flows'); onClose(); } },
      { icon: 'database', label: 'Run a SQL query', act: () => { setView('history'); onClose(); } },
      { icon: 'palette', label: 'Change accent color', act: () => {} },
    ]},
  ]), [setView, onClose]);

  const suggestions = [
    'Why is AHU-3 heating and cooling at the same time?',
    'Show me the worst-performing chillers this week',
    'Which equipment ran after hours yesterday?',
    'Forecast tomorrow’s peak demand for HQ Tower',
  ];

  const filtered = q.trim() ? commands.map(g => ({ ...g, items: g.items.filter(i => i.label.toLowerCase().includes(q.toLowerCase())) })).filter(g => g.items.length) : commands;
  const flat = filtered.flatMap(g => g.items);
  const looksLikeQuestion = q.trim().length > 0 && flat.length === 0;

  const askAgent = (question) => { setQ(question); setMode('agent'); };

  const onKey = (e) => {
    if (mode === 'agent') return;
    if (e.key === 'ArrowDown') { e.preventDefault(); setActiveIdx(i => Math.min(i + 1, flat.length - 1)); }
    else if (e.key === 'ArrowUp') { e.preventDefault(); setActiveIdx(i => Math.max(i - 1, 0)); }
    else if (e.key === 'Enter') {
      e.preventDefault();
      if (looksLikeQuestion || (q.trim() && e.metaKey)) askAgent(q.trim());
      else flat[activeIdx]?.act();
    }
  };

  if (!open) return null;
  return (
    <div onClick={onClose} style={{
      position: 'fixed', inset: 0, zIndex: 100, display: 'flex', alignItems: 'flex-start', justifyContent: 'center',
      paddingTop: '11vh', background: 'color-mix(in oklch, var(--background) 40%, rgba(0,0,0,0.55))', animation: 'overlay-in .12s ease',
    }}>
      <div onClick={e => e.stopPropagation()} className="anim-pop scroll" style={{
        width: 'min(680px, 92vw)', maxHeight: '74vh', background: 'var(--popover)', borderRadius: 16,
        border: '1px solid var(--border-strong)', boxShadow: 'var(--shadow-xl)', overflow: 'hidden', display: 'flex', flexDirection: 'column',
      }}>
        {/* input row */}
        <div style={{ display: 'flex', alignItems: 'center', gap: 11, padding: '14px 16px', borderBottom: '1px solid var(--border)' }}>
          {mode === 'agent'
            ? <button className="btn btn-ghost btn-icon btn-sm" onClick={() => setMode('command')}><Icon name="chevron-left" size={18} /></button>
            : <Icon name="sparkles" size={19} style={{ color: 'var(--primary)' }} />}
          <input ref={inputRef} value={q} onChange={e => { setQ(e.target.value); setActiveIdx(0); }} onKeyDown={onKey}
            placeholder="Ask awaken anything, or type a command…" style={{
              flex: 1, background: 'none', border: 'none', outline: 'none', fontSize: 16, color: 'var(--foreground)', letterSpacing: '-0.01em',
            }} disabled={mode === 'agent'} />
          {mode === 'command' && <span className="badge badge-outline" style={{ gap: 6 }}>
            <Icon name="corner-down-left" size={12} /> {looksLikeQuestion ? 'Ask awaken' : 'Run'}
          </span>}
          <button onClick={onClose} className="kbd" style={{ cursor: 'pointer' }}>esc</button>
        </div>

        {/* body */}
        <div className="scroll" style={{ overflowY: 'auto', flex: 1 }}>
          {mode === 'agent'
            ? <AgentConversation question={q} />
            : <CommandList filtered={filtered} flat={flat} activeIdx={activeIdx} setActiveIdx={setActiveIdx}
                looksLikeQuestion={looksLikeQuestion} q={q} suggestions={suggestions} askAgent={askAgent} />}
        </div>

        {mode === 'command' && <div style={{ display: 'flex', alignItems: 'center', gap: 14, padding: '9px 16px', borderTop: '1px solid var(--border)', fontSize: 11.5, color: 'var(--muted-foreground)' }}>
          <span style={{ display: 'flex', alignItems: 'center', gap: 6 }}><span className="kbd">↑</span><span className="kbd">↓</span> navigate</span>
          <span style={{ display: 'flex', alignItems: 'center', gap: 6 }}><span className="kbd">↵</span> select</span>
          <span style={{ display: 'flex', alignItems: 'center', gap: 6, marginLeft: 'auto' }}><Icon name="sparkles" size={13} style={{ color: 'var(--primary)' }} /> powered by awaken</span>
        </div>}
      </div>
    </div>
  );
}

function CommandList({ filtered, flat, activeIdx, setActiveIdx, looksLikeQuestion, q, suggestions, askAgent }) {
  let idx = -1;
  return (
    <div style={{ padding: 8 }}>
      {looksLikeQuestion && (
        <button onClick={() => askAgent(q.trim())} className="anim-fade" style={{
          display: 'flex', alignItems: 'center', gap: 12, width: '100%', textAlign: 'left', padding: '12px 12px', borderRadius: 10,
          background: 'var(--accent-bg)', border: '1px solid color-mix(in oklch, var(--primary) 30%, transparent)', marginBottom: 6,
        }}>
          <span style={{ width: 32, height: 32, borderRadius: 9, flex: 'none', display: 'grid', placeItems: 'center', background: 'var(--primary)', color: 'var(--primary-fg)' }}><Icon name="sparkles" size={17} /></span>
          <div style={{ flex: 1, minWidth: 0 }}>
            <div style={{ fontSize: 13.5, fontWeight: 600 }}>Ask awaken</div>
            <div className="muted" style={{ fontSize: 12, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>“{q.trim()}”</div>
          </div>
          <Icon name="arrow-right" size={16} style={{ color: 'var(--primary)' }} />
        </button>
      )}

      {!q.trim() && (
        <div style={{ marginBottom: 6 }}>
          <MenuLabel>Ask awaken</MenuLabel>
          {suggestions.map((s, i) => (
            <button key={i} onClick={() => askAgent(s)} style={{
              display: 'flex', alignItems: 'center', gap: 10, width: '100%', textAlign: 'left', padding: '8px 10px', borderRadius: 8, fontSize: 13.5,
            }} onMouseEnter={e => e.currentTarget.style.background = 'var(--accent-bg)'} onMouseLeave={e => e.currentTarget.style.background = 'transparent'}>
              <Icon name="message-square" size={15} style={{ color: 'var(--primary)' }} />
              <span style={{ flex: 1, color: 'var(--foreground)' }}>{s}</span>
              <Icon name="arrow-up-right" size={14} style={{ color: 'var(--muted-foreground)' }} />
            </button>
          ))}
        </div>
      )}

      {flat.length === 0 && q.trim() && !looksLikeQuestion && (
        <div style={{ padding: '28px 12px', textAlign: 'center', color: 'var(--muted-foreground)', fontSize: 13 }}>No commands found.</div>
      )}

      {filtered.map(g => (
        <div key={g.group} style={{ marginBottom: 4 }}>
          <MenuLabel>{g.group}</MenuLabel>
          {g.items.map(it => {
            idx++; const active = idx === activeIdx; const myIdx = idx;
            return (
              <button key={it.label} onClick={it.act} onMouseEnter={() => setActiveIdx(myIdx)} style={{
                display: 'flex', alignItems: 'center', gap: 11, width: '100%', textAlign: 'left', padding: '8px 10px', borderRadius: 8, fontSize: 13.5,
                background: active ? 'var(--accent-bg)' : 'transparent', fontWeight: 500,
              }}>
                <Icon name={it.icon} size={16} style={{ color: active ? 'var(--primary)' : 'var(--muted-foreground)' }} />
                <span style={{ flex: 1, color: 'var(--foreground)' }}>{it.label}</span>
                {it.kbd && <span style={{ display: 'flex', gap: 3 }}>{it.kbd.split(' ').map(k => <span key={k} className="kbd">{k}</span>)}</span>}
              </button>
            );
          })}
        </div>
      ))}
    </div>
  );
}

/* ---------- agent conversation ---------- */
function AgentConversation({ question }) {
  const run = AGENT_RUN;
  const [revealed, setRevealed] = React.useState(0);
  const [decision, setDecision] = React.useState(null); // approved | rejected
  const total = run.steps.length;

  React.useEffect(() => {
    setRevealed(0); setDecision(null);
    let i = 0;
    const tick = () => {
      i++; setRevealed(i);
      if (i < total) {
        const delay = run.steps[i - 1].type === 'tool' ? 520 : run.steps[i - 1].type === 'thinking' ? 900 : 680;
        timer = setTimeout(tick, delay);
      }
    };
    let timer = setTimeout(tick, 650);
    return () => clearTimeout(timer);
  }, [question]);

  return (
    <div style={{ padding: '16px 18px 22px' }}>
      {/* user question */}
      <div style={{ display: 'flex', gap: 11, marginBottom: 16 }}>
        <Avatar name="Dana Okafor" size={28} />
        <div style={{ flex: 1, paddingTop: 3, fontSize: 14, fontWeight: 500 }}>{question}</div>
      </div>

      <div style={{ display: 'flex', flexDirection: 'column', gap: 11 }}>
        {run.steps.slice(0, revealed).map((s, i) => <AgentStep key={i} step={s} decision={decision} setDecision={setDecision} />)}
        {revealed < total && (
          <div style={{ display: 'flex', alignItems: 'center', gap: 9, color: 'var(--muted-foreground)', fontSize: 13, paddingLeft: 2 }}>
            <span className="spinner" /> <span style={{ animation: 'pulse-soft 1.4s infinite' }}>awaken is working…</span>
          </div>
        )}
      </div>
    </div>
  );
}

function StepShell({ icon, iconColor, label, children, tone }) {
  return (
    <div className="anim-slide" style={{ display: 'flex', gap: 11 }}>
      <div style={{ width: 28, flex: 'none', display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 4 }}>
        <span style={{ width: 26, height: 26, borderRadius: 8, display: 'grid', placeItems: 'center', background: tone || 'var(--muted)', color: iconColor || 'var(--muted-foreground)', flex: 'none' }}>
          <Icon name={icon} size={15} />
        </span>
      </div>
      <div style={{ flex: 1, minWidth: 0, paddingTop: 1 }}>
        {label && <div style={{ fontSize: 11, fontWeight: 600, color: 'var(--muted-foreground)', letterSpacing: '0.02em', marginBottom: 4, textTransform: 'uppercase' }}>{label}</div>}
        {children}
      </div>
    </div>
  );
}

function AgentStep({ step, decision, setDecision }) {
  if (step.type === 'trigger') return (
    <StepShell icon="zap" iconColor="var(--sev-fault)" tone="color-mix(in oklch, var(--sev-fault) 14%, transparent)" label="Triggered by">
      <div style={{ fontSize: 13.5 }}>{step.text}</div>
    </StepShell>
  );
  if (step.type === 'thinking') return (
    <StepShell icon="sparkles" iconColor="var(--primary)" tone="var(--accent-bg)">
      <div style={{ fontSize: 13.5, color: 'var(--muted-foreground)', fontStyle: 'italic' }}>{step.text}</div>
    </StepShell>
  );
  if (step.type === 'tool') return (
    <StepShell icon="terminal" label="Tool call">
      <div className="card" style={{ padding: '8px 11px', display: 'flex', alignItems: 'center', gap: 10, background: 'var(--subtle)' }}>
        <span className="badge badge-primary mono" style={{ fontSize: 11 }}>{step.tool}</span>
        <span className="mono" style={{ fontSize: 11.5, color: 'var(--muted-foreground)', flex: 1, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{step.args}</span>
        <span style={{ display: 'flex', alignItems: 'center', gap: 6, flex: 'none' }}>
          <Icon name="check" size={13} style={{ color: 'var(--positive)' }} stroke={2.5} />
          <span className="mono" style={{ fontSize: 11.5, fontWeight: 600 }}>{step.result}</span>
        </span>
      </div>
    </StepShell>
  );
  if (step.type === 'finding') return (
    <StepShell icon="search" label="Diagnosis">
      <div style={{ fontSize: 13.5, lineHeight: 1.55 }}>{step.text}</div>
    </StepShell>
  );
  if (step.type === 'proposal') return (
    <StepShell icon="shield-check" iconColor="var(--sev-warning)" tone="color-mix(in oklch, var(--sev-warning) 16%, transparent)" label="Proposed fix · needs approval">
      <div style={{ fontSize: 13.5, lineHeight: 1.55, marginBottom: 10 }}>{step.text}</div>
      <div className="card" style={{ overflow: 'hidden' }}>
        <div style={{ padding: '8px 12px', borderBottom: '1px solid var(--border)', display: 'flex', alignItems: 'center', gap: 8, fontSize: 11.5, fontWeight: 600, color: 'var(--muted-foreground)' }}>
          <Icon name="hand" size={14} /> Human-in-the-loop · 2 gated point writes
        </div>
        {step.writes.map((w, i) => (
          <div key={i} style={{ display: 'flex', alignItems: 'center', gap: 10, padding: '10px 12px', borderBottom: i < step.writes.length - 1 ? '1px solid var(--border)' : 'none' }}>
            <span className="badge badge-muted" style={{ textTransform: 'capitalize' }}>{w.action}</span>
            <span className="mono" style={{ fontSize: 12, flex: 1, minWidth: 0, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{w.point}</span>
            <span className="mono" style={{ fontSize: 12, color: 'var(--muted-foreground)' }}>{w.from}</span>
            <Icon name="arrow-right" size={13} style={{ color: 'var(--muted-foreground)' }} />
            <span className="mono" style={{ fontSize: 12, fontWeight: 700, color: 'var(--foreground)' }}>{w.to}</span>
            <span className="badge badge-outline" style={{ flex: 'none' }}>prio {w.priority}</span>
          </div>
        ))}
        <div style={{ display: 'flex', alignItems: 'center', gap: 8, padding: '10px 12px', background: 'var(--subtle)' }}>
          {decision === 'approved' ? (
            <span className="badge bg-sev-info" style={{ background: 'color-mix(in oklch, var(--positive) 15%, transparent)', color: 'var(--positive)', gap: 6, height: 26, padding: '0 10px' }}>
              <Icon name="check-check" size={14} /> Approved · writes dispatched at priority 13
            </span>
          ) : decision === 'rejected' ? (
            <span className="badge badge-muted" style={{ gap: 6, height: 26, padding: '0 10px' }}><Icon name="x" size={14} /> Rejected · no writes made</span>
          ) : (
            <>
              <button className="btn btn-primary btn-sm" onClick={() => setDecision('approved')}><Icon name="check" size={14} stroke={2.5} /> Approve & write</button>
              <button className="btn btn-outline btn-sm" onClick={() => setDecision('rejected')}>Reject</button>
              <button className="btn btn-ghost btn-sm" style={{ marginLeft: 'auto' }}><Icon name="square-pen" size={14} /> Edit</button>
            </>
          )}
        </div>
      </div>
    </StepShell>
  );
  return null;
}

Object.assign(window, { CommandPalette });
