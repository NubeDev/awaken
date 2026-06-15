# QUICKSTART — Boot the server, seed demo data, first API call

> Verified: WS-16 (2026-06-15)
> Run this once the HTTP transport layer (WS-16) lands and tests are green.

---

## Prerequisites

- ✅ `cargo test --workspace` passes
- ✅ `cargo clippy --workspace --all-targets` passes
- ✅ Port 8080 is free (or override `RUBIX_BIND`)
- ✅ The data dir (`rubix-data/`) doesn't exist (fresh start) or can be deleted

There is **no HTTP endpoint to create principals or grants** — identity precedes
any scoped session, so it is provisioned in-process. Use the `--seed-dev` flag
(below) to populate a demo portfolio plus a ready-to-use cast of principals.

---

## 1. Start the server with the demo seed

```bash
cd rubix

# Clean up old data (deterministic ids assume a fresh store)
rm -rf rubix-data

# Boot the server and seed the demo portfolio
make dev-be SEED=1
# or directly:
cargo run --bin rubix-server -- --seed-dev
```

The seed prints a login table and per-tenant tallies, then the server serves:

```
seeding demo portfolio (2 tenants)
subject              secret         grants
acme_operator        operator-demo  ingest-publish
acme_viewer          viewer-demo    —
acme_analyst         analyst-demo   external-query
acme_agent           agent-demo     external-query,rule-invoke
globex_operator      operator-demo  ingest-publish
...
seed complete: 1320 records across 2 tenants
```

### What gets seeded

Two **tenants** (`acme`, `globex`), each with two **sites**, each site carrying
**HVAC, energy, and water** equipment as Project-Haystack-style records connected
by the tag graph (`site → equip → point → reading`), plus 24h of hourly
readings per point. The "tenant" is the principal's `namespace`; reads are
confined to it by SurrealDB row-level permissions (no cloud profile needed).

Per tenant the cast covers both authz layers: `operator` (writes, holds
`ingest-publish`), `viewer` (read-only), `analyst` (`external-query` for SQL),
and `agent` (an extension service account with `external-query` + `rule-invoke`).

The server boots empty without `SEED=1` / `--seed-dev`.

---

## 2. Check health

In another terminal:

```bash
curl http://127.0.0.1:8080/health
```

Expected:

```json
{ "status": "ok" }
```

✅ **Server is running.** Set `BASE=http://127.0.0.1:8080` for the calls below.

---

## 3. Authenticate

Every authenticated route reads two credential headers — the principal's subject
and secret (no JWT/cookie layer; the gate's record access method verifies the
pair natively):

```
-H "x-rubix-subject: acme_operator" -H "x-rubix-secret: operator-demo"
```

A missing/wrong credential is `401`; a present principal lacking the required
capability grant is `403`.

---

## 4. Read a record (scoped session)

Reads run on the principal's scoped session, so they only ever return the
principal's own namespace:

```bash
curl "$BASE/records/acme--hq" \
  -H "x-rubix-subject: acme_viewer" -H "x-rubix-secret: viewer-demo"
```

Expected (a seeded site record):

```json
{
  "id": "acme--hq",
  "namespace": "acme",
  "content": { "kind": "site", "key": "hq", "name": "Acme HQ" },
  "created": "...",
  "updated": "..."
}
```

A `globex` principal fetching this `acme` id gets `404` — tenant isolation is
enforced by the engine, not the app.

---

## 5. Create a record (through the command gate)

A write is a mutation, so it crosses the WS-05 gate and needs the
`ingest-publish` grant — the `operator` has it:

```bash
curl -X POST "$BASE/records" \
  -H "content-type: application/json" \
  -H "x-rubix-subject: acme_operator" -H "x-rubix-secret: operator-demo" \
  -d '{ "content": { "kind": "note", "name": "manual entry" } }'
```

The id is minted server-side; the gate writes it under the principal's namespace,
captures before/after, mints a correlation id, and appends an audit row.

Trying the same call as `acme_viewer` (no grant) returns `403`.

---

## 6. Query records (DataFusion)

The unified SQL surface is gated on the `external-query` capability (the
`analyst` holds it) and runs on the scoped session. The table is named
**`record`** (singular, matching the store table); its columns are
`id, namespace, created, updated, content` — `content` is the JSON document as a
string, reached into for field access:

```bash
curl -X POST "$BASE/query" \
  -H "content-type: application/json" \
  -H "x-rubix-subject: acme_analyst" -H "x-rubix-secret: analyst-demo" \
  -d '{ "sql": "SELECT count(*) AS readings FROM record WHERE content LIKE '\''%\"kind\":\"reading\"%'\''" }'
```

Expected:

```json
{ "rows": [ { "readings": 624 } ] }
```

Running `/query` as `acme_viewer` (no `external-query` grant) returns `403`.

✅ **Query surface working, scoped to the tenant.**

---

## 7. Stop the server

Press `Ctrl-C` (or `make kill` to free the ports).

---

## ✅ Checklist: Server is ready

- [x] Server boots without errors
- [x] `--seed-dev` populates the demo portfolio (1320 records, 2 tenants)
- [x] Health endpoint responds
- [x] Auth resolves the seeded credentials (`401` on bad secret)
- [x] Record read works and is tenant-scoped (`404` cross-tenant)
- [x] Record write works through the gate (`403` without the grant)
- [x] Query works (DataFusion over `record`, gated on `external-query`)

**Next:** Pick a feature runbook and exercise the end-to-end flow.

---

## Troubleshooting

### ❌ "Address already in use"

Port 8080 is taken.

**Fix:** Free it or override the bind address:

```bash
make kill
# or
RUBIX_BIND=127.0.0.1:9999 cargo run --bin rubix-server
```

### ❌ "unauthenticated: ... No record was returned"

The subject/secret pair doesn't resolve — check the headers against the login
table the seed printed (subjects are `{tenant}_{role}`, e.g. `acme_analyst`).

### ❌ Seeding errors on a non-fresh store

The seed uses deterministic record ids, so re-seeding over existing data can
collide. Delete the data dir and re-run:

```bash
rm -rf rubix-data && make dev-be SEED=1
```

### ❌ "table 'records' not found"

The DataFusion table is `record` (singular), not `records`.

---

## Next steps

- **Feature testing:** Pick a feature doc (gate, datasources, rules, query)
- **Live WebSocket:** Test the `/ws` bridge for live updates while writing records
- **Integration scenarios:** Run the golden-path cross-feature scripts in `scenarios/`
