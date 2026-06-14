# Evidence Capture — The Standard Bundle

> When a ✅ check fails, capture this **before** theorizing. A consistent bundle
> makes triage mechanical and lets a fresh AI session reason without re-running.

Output dir: `testing/.evidence/<scenario>/<timestamp>/` (git-ignored). Use one dir
per failure so before/after fixes are comparable.

---

## What goes in the bundle

| Artifact | How | Why |
|----------|-----|-----|
| `symptom.md` | one paragraph: which doc, which step, expected vs actual | the question being answered |
| `server.log` | the `rubix-server` stdout/stderr around the failure | the primary signal (incl. driver spawn/attach lines) |
| `point.json` | `GET /api/v1/points/{id}` | `cur_value`/`cur_ts`/`priority_array` state |
| `query_count.json` | a `count(*)` over `his`/`points`/`sparks` | did data land |
| `db_state.txt` | direct SQLite row counts (bypasses DataFusion) | store ground truth |
| `request.txt` | the exact curl that failed + full response (status + body) | reproduction |
| `env.txt` | the `RUBIX_*` env vars (redact keys) + git commit | what was running |
| `openapi_slice.json` | the relevant path from `/api-docs/openapi.json` | contract vs reality |
| `drivers.json` | the manifest in effect | scoping / spawn config |

---

## One-shot capture (adapt to your shell)

```bash
SCN=${1:-adhoc}; TS=$(date +%Y%m%d-%H%M%S)
DIR=testing/.evidence/$SCN/$TS; mkdir -p "$DIR"
BASE=${BASE:-http://127.0.0.1:8088}    # make dev-be default; raw cargo run uses :8080

# context
( git rev-parse HEAD; echo "---"; env | grep -E '^RUBIX_' \
  | sed -E 's/(KEY|TOKEN|SECRET)=.*/\1=<redacted>/' ) > "$DIR/env.txt"
cp drivers.json "$DIR/drivers.json" 2>/dev/null || true

# api state
[ -n "$POINT" ] && curl -s $BASE/api/v1/points/$POINT > "$DIR/point.json"
curl -s -X POST $BASE/api/v1/query -H content-type:application/json \
  -d '{"sql":"SELECT (SELECT count(*) FROM his) AS his, (SELECT count(*) FROM points) AS points, (SELECT count(*) FROM sparks) AS sparks"}' \
  > "$DIR/query_count.json"

# store ground truth — direct SQLite, bypassing DataFusion/feature gates
sqlite3 "${RUBIX_DB:-rubix.db}" \
  'SELECT "his",count(*) FROM his UNION ALL SELECT "points",count(*) FROM points UNION ALL SELECT "sparks",count(*) FROM sparks;' \
  > "$DIR/db_state.txt" 2>&1

# contract slice
curl -s $BASE/api-docs/openapi.json | jq ".paths" > "$DIR/openapi_slice.json"

echo "bundle → $DIR"
```

Then drop in `server.log` (copy the relevant window — include the driver spawn /
liveliness lines and the few lines *before* the error), `request.txt` (the failing
curl + response), and write `symptom.md`.

> The reusable form of the above lives at `testing/scripts/capture.sh` (git-ignored):
> `testing/scripts/capture.sh <scenario>` writes the bundle and echoes its dir;
> `POINT=<id>` adds `point.json`, `SYMPTOM='…'` seeds `symptom.md`. Drop `server.log`
> and `request.txt` in afterwards. `run-scenario.sh` calls it automatically on a ❌.

---

## Quality bar for the bundle

- The failing request is **reproducible** from `request.txt` alone.
- `db_state.txt` is the **direct SQLite** count — it tells you whether the data is
  in the store regardless of whether `/query` is enabled or a view is wrong. A
  mismatch between `query_count.json` and `db_state.txt` localizes the bug to the
  query layer vs the write path.
- `server.log` includes the error *and* the lines before it (the cause is usually
  upstream — a driver that failed to attach, a keyexpr mismatch, a scope denial).
- The git commit is recorded — a fix is meaningless without knowing the baseline.
