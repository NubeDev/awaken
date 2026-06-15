#!/usr/bin/env bash
# Full end-to-end test of the rubix server: seed, login/auth, records CRUD,
# capability gating, tenant isolation, query surface, datasources, openapi, ws.
#
# Usage (from anywhere):
#   docs/testing/scenarios/e2e.sh            # defaults: port 8097, /tmp data+log
#   PORT=9000 docs/testing/scenarios/e2e.sh  # override the port
#
# Boots `rubix-server --seed-dev` on a throwaway file-backed store, runs every
# check, prints "RESULT: N passed, M failed", then tears the server down.
set -u
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RUBIX_DIR="$(cd "$SCRIPT_DIR/../../.." && pwd)"   # docs/testing/scenarios -> rubix/
cd "$RUBIX_DIR"

PORT="${PORT:-8097}"
B="http://127.0.0.1:$PORT"
LOG="${LOG:-/tmp/rubix-e2e.log}"
DATA="${DATA:-/tmp/rubix-e2e}"

pkill -9 -f 'rubix-server' 2>/dev/null
sleep 1
rm -rf "$DATA"

echo "### booting server (seed) ..."
RUBIX_DATA_DIR="$DATA" RUBIX_BIND="127.0.0.1:$PORT" \
  cargo run -q -p rubix-server --bin rubix-server -- --seed-dev > "$LOG" 2>&1 &
SRV=$!

# Wait for the server to finish seeding and start serving.
for i in $(seq 1 150); do
  curl -s "$B/health" >/dev/null 2>&1 && break
  if ! kill -0 "$SRV" 2>/dev/null; then echo "!!! server died during boot"; cat "$LOG"; exit 1; fi
  sleep 2
done
echo "### seed log tail:"; tail -3 "$LOG"

PASS=0; FAIL=0
# check NAME EXPECTED ACTUAL
ck() { if [ "$2" = "$3" ]; then PASS=$((PASS+1)); echo "PASS  $1 ($3)"; else FAIL=$((FAIL+1)); echo "FAIL  $1  expected=$2 got=$3"; fi; }
# http status for a request: ck_code NAME EXPECTED curl-args...
ck_code() { local n="$1" exp="$2"; shift 2; local code; code=$(curl -s -o /dev/null -w '%{http_code}' "$@"); ck "$n" "$exp" "$code"; }

OP=(-H 'x-rubix-subject: acme_operator' -H 'x-rubix-secret: operator-demo')
VW=(-H 'x-rubix-subject: acme_viewer'   -H 'x-rubix-secret: viewer-demo')
AN=(-H 'x-rubix-subject: acme_analyst'  -H 'x-rubix-secret: analyst-demo')
AG=(-H 'x-rubix-subject: acme_agent'    -H 'x-rubix-secret: agent-demo')
GVW=(-H 'x-rubix-subject: globex_viewer' -H 'x-rubix-secret: viewer-demo')
GAN=(-H 'x-rubix-subject: globex_analyst' -H 'x-rubix-secret: analyst-demo')
JSON=(-H 'content-type: application/json')

echo; echo "===== A. HEALTH ====="
ck "health.status" "ok" "$(curl -s $B/health | jq -r .status)"

echo; echo "===== B. AUTH ====="
ck_code "auth.valid.operator"     200 "$B/records" "${OP[@]}"
ck_code "auth.missing.both"       401 "$B/records"
ck_code "auth.missing.secret"     401 "$B/records" -H 'x-rubix-subject: acme_viewer'
ck_code "auth.unknown.subject"    401 "$B/records" -H 'x-rubix-subject: nobody' -H 'x-rubix-secret: x'
ck_code "auth.bad.secret"         401 "$B/records" -H 'x-rubix-subject: acme_viewer' -H 'x-rubix-secret: wrong'
ck_code "auth.extension.agent"    200 "$B/records" "${AG[@]}"

echo; echo "===== C. RECORDS READ + TENANT ISOLATION ====="
ck "read.site.ns"        "acme" "$(curl -s $B/records/acme--hq "${VW[@]}" | jq -r .namespace)"
ck "read.site.kind"      "site" "$(curl -s $B/records/acme--hq "${VW[@]}" | jq -r .content.kind)"
ck_code "read.crosstenant.404"   404 "$B/records/acme--hq" "${GVW[@]}"
ck_code "read.missing.404"        404 "$B/records/does-not-exist" "${VW[@]}"
ck "list.acme.count"     "660" "$(curl -s $B/records "${VW[@]}" | jq 'length')"
ck "list.globex.count"   "660" "$(curl -s $B/records "${GVW[@]}" | jq 'length')"

echo; echo "===== D. RECORDS WRITE (gate + capability) ====="
NEW=$(curl -s -X POST $B/records "${OP[@]}" "${JSON[@]}" -d '{"content":{"kind":"note","name":"e2e"}}')
NID=$(echo "$NEW" | jq -r .id)
ck "write.create.ns"     "acme" "$(echo "$NEW" | jq -r .namespace)"
ck "write.create.hasid"  "true" "$([ -n "$NID" ] && [ "$NID" != "null" ] && echo true || echo false)"
ck_code "write.create.viewer.403"   403 -X POST "$B/records" "${VW[@]}" "${JSON[@]}" -d '{"content":{}}'
ck_code "write.create.unauth.401"   401 -X POST "$B/records" "${JSON[@]}" -d '{"content":{}}'
# update
ck_code "write.update.operator.200" 200 -X PATCH "$B/records/$NID" "${OP[@]}" "${JSON[@]}" -d '{"content":{"kind":"note","name":"e2e-updated"}}'
ck "write.update.applied" "e2e-updated" "$(curl -s $B/records/$NID "${VW[@]}" | jq -r .content.name)"
ck_code "write.update.viewer.403"   403 -X PATCH "$B/records/$NID" "${VW[@]}" "${JSON[@]}" -d '{"content":{}}'
# delete
ck_code "write.delete.viewer.403"   403 -X DELETE "$B/records/$NID" "${VW[@]}"
DEL=$(curl -s -o /dev/null -w '%{http_code}' -X DELETE "$B/records/$NID" "${OP[@]}")
ck "write.delete.operator.2xx" "2xx" "$([ "$DEL" = 200 ] || [ "$DEL" = 204 ] && echo 2xx || echo $DEL)"
ck_code "write.delete.gone.404"     404 "$B/records/$NID" "${VW[@]}"

echo; echo "===== E. QUERY (DataFusion, external-query) ====="
ck "query.count.acme"    "660" "$(curl -s -X POST $B/query "${AN[@]}" "${JSON[@]}" -d '{"sql":"SELECT count(*) AS n FROM record"}' | jq '.rows[0].n')"
ck "query.count.globex"  "660" "$(curl -s -X POST $B/query "${GAN[@]}" "${JSON[@]}" -d '{"sql":"SELECT count(*) AS n FROM record"}' | jq '.rows[0].n')"
ck "query.readings"      "624" "$(curl -s -X POST $B/query "${AN[@]}" "${JSON[@]}" -d '{"sql":"SELECT count(*) AS n FROM record WHERE content LIKE '"'"'%\"kind\":\"reading\"%'"'"'"}' | jq '.rows[0].n')"
ck_code "query.viewer.no-cap.403"   403 -X POST "$B/query" "${VW[@]}" "${JSON[@]}" -d '{"sql":"SELECT 1"}'
ck_code "query.operator.no-cap.403" 403 -X POST "$B/query" "${OP[@]}" "${JSON[@]}" -d '{"sql":"SELECT 1"}'
ck_code "query.badsql.400"          400 -X POST "$B/query" "${AN[@]}" "${JSON[@]}" -d '{"sql":"NOT SQL AT ALL"}'
ck_code "query.nonselect.4xx"       400 -X POST "$B/query" "${AN[@]}" "${JSON[@]}" -d '{"sql":"DELETE FROM record"}'

echo; echo "===== F. DATASOURCES / OPENAPI ====="
ck_code "datasources.list.200" 200 "$B/datasources" "${VW[@]}"
ck_code "openapi.json.200"     200 "$B/api-docs/openapi.json"
ck "openapi.has.records" "true" "$(curl -s $B/api-docs/openapi.json | jq '.paths | has("/records")')"

echo; echo "===== G. WS LIVE-QUERY BRIDGE ====="
node "$SCRIPT_DIR/ws-test.js" "$PORT"; WS=$?
ck "ws.live-event" "0" "$WS"

echo; echo "===== POST-RUN ====="
ck_code "post-tests.health.200" 200 "$B/health"

echo
echo "================= RESULT: $PASS passed, $FAIL failed ================="
kill -9 "$SRV" 2>/dev/null
[ "$FAIL" -eq 0 ]
