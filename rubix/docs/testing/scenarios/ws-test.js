// Connect to the live-query bridge, then create a record and assert an event
// arrives. Exit 0 on success, non-zero otherwise.
//
// Usage: node ws-test.js [port]   (default 8097)
//
// The /ws/records route authenticates via the same x-rubix-subject/x-rubix-secret
// headers as the HTTP routes. Node's built-in WebSocket accepts a { headers }
// option (a browser WebSocket cannot — see TEST-STATUS.md "known gap").
const port = process.argv[2] || '8097';
const base = `http://127.0.0.1:${port}`;
const wsUrl = `ws://127.0.0.1:${port}/ws/records`;
const cred = { 'x-rubix-subject': 'acme_operator', 'x-rubix-secret': 'operator-demo' };
const headers = { ...cred, 'content-type': 'application/json' };

const ws = new WebSocket(wsUrl, { headers: cred });
let got = false;
const timer = setTimeout(() => { console.error('timeout waiting for ws event'); process.exit(2); }, 8000);

ws.addEventListener('open', async () => {
  console.error('ws open; creating record to trigger event');
  try {
    const r = await fetch(`${base}/records`, { method: 'POST', headers, body: JSON.stringify({ content: { kind: 'note', name: 'ws-trigger' } }) });
    console.error('create status', r.status);
  } catch (e) { console.error('create failed', e.message); }
});
ws.addEventListener('message', (ev) => {
  got = true;
  const s = typeof ev.data === 'string' ? ev.data : '[binary]';
  console.error('event:', s.slice(0, 120));
  clearTimeout(timer);
  ws.close();
  process.exit(0);
});
ws.addEventListener('error', (e) => { console.error('ws error', e.message || e); });
ws.addEventListener('close', () => { if (!got) { console.error('closed without event'); process.exit(3); } });
