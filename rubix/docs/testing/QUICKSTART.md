# QUICKSTART — Boot the server and first API call

> Verified: WS-16 (2026-06-15)
> Run this once the HTTP transport layer (WS-16) lands and tests are green.

---

## Prerequisites

- ✅ `cargo test --workspace` passes
- ✅ `cargo clippy --workspace --all-targets` passes
- ✅ Port 8088 is free (or override `RUBIX_ADDR`)
- ✅ `rubix.db` doesn't exist (fresh start) or can be deleted

---

## 1. Start the server

```bash
cd rubix

# Clean up old database (if any)
rm -f rubix.db

# Boot the server
cargo run --bin rubix
# or
make dev-be
```

Expected output:

```
2026-06-15T00:30:00Z INFO rubix_server: starting rubix backend
2026-06-15T00:30:00Z INFO rubix_store: initializing SurrealDB (embedded kv-mem)
2026-06-15T00:30:00Z INFO rubix_server: listening on 127.0.0.1:8088
2026-06-15T00:30:00Z INFO rubix_server: health check: OK
```

The server is ready when you see `listening on 127.0.0.1:8088`.

---

## 2. Check health

In another terminal:

```bash
curl http://127.0.0.1:8088/health
```

Expected:

```json
{
  "status": "ok",
  "timestamp": "2026-06-15T00:30:00Z",
  "version": "0.1.0"
}
```

✅ **Server is running.**

---

## 3. Create a principal (identity)

Records are scoped by namespace. Create one:

```bash
curl -X POST http://127.0.0.1:8088/api/principals \
  -H "Content-Type: application/json" \
  -d '{
    "subject": "alice",
    "namespace": "acme",
    "kind": "user",
    "role": "admin"
  }'
```

Expected:

```json
{
  "id": "...",
  "subject": "alice",
  "namespace": "acme",
  "kind": "user",
  "role": "admin"
}
```

Copy the `id` for later; call it `$PRINCIPAL_ID`.

---

## 4. Grant a capability

Alice needs the `ingest-publish` capability to write records:

```bash
curl -X POST http://127.0.0.1:8088/api/grants \
  -H "Content-Type: application/json" \
  -d "{
    \"principal_id\": \"$PRINCIPAL_ID\",
    \"capability\": \"ingest-publish\",
    \"namespace\": \"acme\"
  }"
```

Expected:

```json
{
  "id": "...",
  "principal_id": "$PRINCIPAL_ID",
  "capability": "ingest-publish",
  "namespace": "acme"
}
```

✅ **Alice can now write records in the acme namespace.**

---

## 5. Create a record

Write a record (this goes through the command gate, WS-05):

```bash
curl -X POST http://127.0.0.1:8088/api/records \
  -H "Content-Type: application/json" \
  -H "X-Principal: $PRINCIPAL_ID" \
  -d '{
    "namespace": "acme",
    "content": {
      "name": "Equipment-01",
      "type": "AHU",
      "location": "Building-A"
    }
  }'
```

Expected:

```json
{
  "id": "...",
  "namespace": "acme",
  "content": {
    "name": "Equipment-01",
    "type": "AHU",
    "location": "Building-A"
  },
  "created_at": "2026-06-15T00:30:00Z",
  "updated_at": "2026-06-15T00:30:00Z"
}
```

Copy the `id`; call it `$RECORD_ID`.

---

## 6. Read the record

```bash
curl http://127.0.0.1:8088/api/records/$RECORD_ID \
  -H "X-Principal: $PRINCIPAL_ID"
```

Expected:

```json
{
  "id": "$RECORD_ID",
  "namespace": "acme",
  "content": { ... },
  "created_at": "...",
  "updated_at": "..."
}
```

✅ **Read/write gate working.**

---

## 7. Query records

Run a DataFusion SQL query over the records:

```bash
curl -X POST http://127.0.0.1:8088/api/query \
  -H "Content-Type: application/json" \
  -H "X-Principal: $PRINCIPAL_ID" \
  -d '{
    "sql": "SELECT id, namespace FROM records WHERE namespace = '\''acme'\''"
  }'
```

Expected:

```json
{
  "rows": [
    {
      "id": "$RECORD_ID",
      "namespace": "acme"
    }
  ]
}
```

✅ **Query surface working.**

---

## 8. Verify the database

Check the SurrealDB store:

```bash
# List all records in the acme namespace
curl -X POST http://127.0.0.1:8088/api/query \
  -H "Content-Type: application/json" \
  -H "X-Principal: $PRINCIPAL_ID" \
  -d '{
    "sql": "SELECT COUNT(*) as count FROM records"
  }'
```

Expected:

```json
{
  "rows": [{ "count": 1 }]
}
```

---

## 9. Stop the server

Press `Ctrl-C` in the terminal where the server is running.

Expected:

```
^C
2026-06-15T00:32:00Z INFO rubix_server: shutting down gracefully
2026-06-15T00:32:00Z INFO rubix_server: stopped
```

---

## ✅ Checklist: Server is ready

- [x] Server boots without errors
- [x] Health endpoint responds
- [x] Principal creation works
- [x] Grant assignment works
- [x] Record write works (through the gate)
- [x] Record read works (scoped session)
- [x] Query works (DataFusion)
- [x] Database has data (SurrealDB)

**Next:** Pick a feature runbook and exercise the end-to-end flow.

---

## Troubleshooting

### ❌ "Address already in use"

Port 8088 is taken.

**Fix:** Kill the old process or override the port:

```bash
make kill
# or
RUBIX_ADDR=127.0.0.1:9999 cargo run --bin rubix
```

### ❌ "Database error: cannot write to rubix.db"

SurrealDB file is locked or read-only.

**Fix:** Remove and restart:

```bash
rm rubix.db
cargo run --bin rubix
```

### ❌ "Principal not found" or "Unauthorized"

The principal ID is wrong or missing.

**Fix:** Check the header:

```bash
curl ... -H "X-Principal: <correct-id>"
```

### ❌ "Query returned 0 rows"

The namespace doesn't match or records are in a different namespace.

**Fix:** Query all records:

```bash
curl ... -d '{"sql": "SELECT * FROM records"}'
```

---

## Next steps

- **Feature testing:** Pick a feature doc (gate, datasources, rules, query)
- **Live WebSocket:** Once WS-16 is complete, test the `/ws` bridge for live updates
- **Integration scenarios:** Run the golden-path cross-feature scripts in `scenarios/`
