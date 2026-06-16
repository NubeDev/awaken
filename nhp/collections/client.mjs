// A tiny HTTP client for the rubix records API the registrar POSTs to.
//
// NHP has no backend crate — collection definitions are DATA created over the
// already-built rubix HTTP surface (rubix/crates/rubix-server/src/http/records/).
// Auth is the service-account header pair x-rubix-subject / x-rubix-secret
// (rubix/crates/rubix-server/src/auth.rs); the principal must hold the
// record-write capability (`ingest-publish`). Against a `--seed-dev` server in
// namespace `acme` that is `acme_operator` / `operator-demo`.
//
// Config from env so the same script runs against any rubix server:
//   RUBIX_BASE     base URL              (default http://127.0.0.1:8097)
//   RUBIX_SUBJECT  x-rubix-subject       (default acme_operator)
//   RUBIX_SECRET   x-rubix-secret        (default operator-demo)

const BASE = process.env.RUBIX_BASE ?? 'http://127.0.0.1:8097';
const SUBJECT = process.env.RUBIX_SUBJECT ?? 'acme_operator';
const SECRET = process.env.RUBIX_SECRET ?? 'operator-demo';

const headers = {
  'content-type': 'application/json',
  'x-rubix-subject': SUBJECT,
  'x-rubix-secret': SECRET,
};

// POST /records with `content`. Returns { ok, status, body } — never throws on a
// non-2xx, so a caller can assert on the status (the invalid-record checks rely
// on seeing a 4xx rejection, not an exception).
export async function createRecord(content) {
  const res = await fetch(`${BASE}/records`, {
    method: 'POST',
    headers,
    body: JSON.stringify({ content }),
  });
  const body = await res.json().catch(() => null);
  return { ok: res.ok, status: res.status, body };
}

// GET /records?kind=… → the array of records of that kind the principal can read.
export async function listRecords(kind) {
  const url = new URL(`${BASE}/records`);
  if (kind) url.searchParams.set('kind', kind);
  const res = await fetch(url, { headers });
  if (!res.ok) {
    throw new Error(`GET /records?kind=${kind} → ${res.status}`);
  }
  return res.json();
}

// POST /readings — bulk-append time-series samples for one series (the register
// record id), each `{ at, value }`. This is the DATA plane, not the gate: it gates
// once on `readings-append` and writes append-only with a deterministic
// (series, at) id, so a re-seed is idempotent (no per-sample audit/undo). Returns
// { ok, status, body } so a caller can assert on the status without throwing.
export async function appendReadings(series, samples) {
  const res = await fetch(`${BASE}/readings`, {
    method: 'POST',
    headers,
    body: JSON.stringify({ series, samples }),
  });
  const body = await res.json().catch(() => null);
  return { ok: res.ok, status: res.status, body };
}

// GET /readings?series=&from=&to= → the windowed, series-scoped historian read:
// the array of `{ series, at, value }` samples for one series in `[from, to]`,
// at-ordered. The namespace fence is enforced by the scoped session.
export async function getReadings(series, from, to) {
  const url = new URL(`${BASE}/readings`);
  url.searchParams.set('series', series);
  url.searchParams.set('from', from);
  url.searchParams.set('to', to);
  const res = await fetch(url, { headers });
  if (!res.ok) {
    throw new Error(`GET /readings?series=${series} → ${res.status}`);
  }
  return res.json();
}
