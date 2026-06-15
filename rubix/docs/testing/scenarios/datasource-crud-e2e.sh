#!/usr/bin/env bash
# End-to-end test of the datasource CRUD control plane over the live HTTP API.
#
# Exercises the full Grafana-style "add a datasource" surface against a real
# server + real Postgres: register a Postgres connector through POST /datasources,
# list/get it, run a federated SELECT that spans SurrealDB and the live Postgres
# through POST /query, update it, delete it, verify it is gone, confirm the
# capability gate (viewer is denied), and confirm the registration survives a
# server restart (rehydration from the persisted `datasource` table).
#
# Unlike datasource-e2e.sh (which runs the gated connector unit tests directly),
# this drives the connector entirely through the HTTP control plane — the path a
# UI/client actually uses. Requires the `postgres` server feature, so it builds
# with --features postgres.
#
# Usage (from anywhere):
#   docs/testing/scenarios/datasource-crud-e2e.sh
#   PORT=9001 DB_PORT=5433 docs/testing/scenarios/datasource-crud-e2e.sh
#
# Requires: docker, jq, curl, node not needed.
set -u
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RUBIX_DIR="$(cd "$SCRIPT_DIR/../../.." && pwd)"
cd "$RUBIX_DIR"

PORT="${PORT:-8098}"
B="http://127.0.0.1:$PORT"
DB_PORT="${DB_PORT:-5433}"
export DB_PORT
PGURL="postgres://rubix:rubix@127.0.0.1:${DB_PORT}/rubix?sslmode=disable"
LOG="${LOG:-/tmp/rubix-ds-crud.log}"
DATA="${DATA:-/tmp/rubix-ds-crud}"

PASS=0; FAIL=0
ck() { if [ "$2" = "$3" ]; then PASS=$((PASS+1)); echo "PASS  $1 ($3)"; else FAIL=$((FAIL+1)); echo "FAIL  $1  expected=$2 got=$3"; fi; }
ck_code() { local n="$1" exp="$2"; shift 2; local code; code=$(curl -s -o /dev/null -w '%{http_code}' "$@"); ck "$n" "$exp" "$code"; }

OP=(-H 'x-rubix-subject: acme_operator' -H 'x-rubix-secret: operator-demo')
VW=(-H 'x-rubix-subject: acme_viewer'   -H 'x-rubix-secret: viewer-demo')
AN=(-H 'x-rubix-subject: acme_analyst'  -H 'x-rubix-secret: analyst-demo')
JSON=(-H 'content-type: application/json')

echo "### bringing up TimescaleDB on ${DB_PORT} ..."
make -C "$RUBIX_DIR" db-up >/dev/null 2>&1 || { echo "db-up failed"; exit 1; }

echo "### seeding a Postgres table ..."
docker exec -i rubix-timescaledb psql -U rubix -d rubix >/dev/null 2>&1 <<'SQL'
DROP TABLE IF EXISTS warehouse_readings;
CREATE TABLE warehouse_readings (id int primary key, site text, value double precision);
INSERT INTO warehouse_readings VALUES (1,'hq',10.5),(2,'hq',11.5),(3,'plant',20.0);
SQL

echo "### building server (--features postgres) ..."
cargo build -q -p rubix-server --bin rubix-server --features postgres 2>&1 | tail -3

boot() {
  pkill -9 -f 'rubix-server' 2>/dev/null; sleep 1
  RUBIX_DATA_DIR="$DATA" RUBIX_BIND="127.0.0.1:$PORT" \
    cargo run -q -p rubix-server --bin rubix-server --features postgres -- "$@" > "$LOG" 2>&1 &
  SRV=$!
  # The first (seeded) boot writes 1320 records on a debug build; under a busy
  # box that can take a few minutes, so poll generously before giving up.
  for i in $(seq 1 300); do
    curl -s "$B/health" >/dev/null 2>&1 && return 0
    kill -0 "$SRV" 2>/dev/null || { echo "!!! server died during boot"; cat "$LOG"; return 1; }
    sleep 2
  done
  echo "!!! server never came up (last log lines:)"; tail -5 "$LOG"; return 1
}

rm -rf "$DATA"
echo "### booting server (seed) ..."
boot --seed-dev || exit 1

echo; echo "===== A. EMPTY STATE (native only) ====="
ck "list.native.only"   "1"          "$(curl -s $B/datasources "${VW[@]}" | jq 'length')"
ck "list.native.id"     "surrealdb"  "$(curl -s $B/datasources "${VW[@]}" | jq -r '.[0].id')"
ck "list.native.kind"   "surrealdb"  "$(curl -s $B/datasources "${VW[@]}" | jq -r '.[0].kind')"

echo; echo "===== B. REGISTER (POST, capability-gated) ====="
REG=$(printf '{"id":"warehouse","label":"Cloud Warehouse","kind":"postgres","connection_string":"%s","tables":["warehouse_readings"]}' "$PGURL")
ck_code "register.viewer.403"   403 -X POST "$B/datasources" "${VW[@]}" "${JSON[@]}" -d "$REG"
ck_code "register.unauth.401"   401 -X POST "$B/datasources" "${JSON[@]}" -d "$REG"
NEW=$(curl -s -X POST "$B/datasources" "${OP[@]}" "${JSON[@]}" -d "$REG")
ck "register.operator.id"   "warehouse"  "$(echo "$NEW" | jq -r .id)"
ck "register.operator.kind" "postgres"   "$(echo "$NEW" | jq -r .kind)"
ck "register.no-secret-leak" "null"      "$(echo "$NEW" | jq -r '.connection_string')"
ck_code "register.duplicate.409" 409 -X POST "$B/datasources" "${OP[@]}" "${JSON[@]}" -d "$REG"
BADKIND='{"id":"x","label":"x","kind":"mysql","connection_string":"x","tables":[]}'
ck_code "register.badkind.400"   400 -X POST "$B/datasources" "${OP[@]}" "${JSON[@]}" -d "$BADKIND"

echo; echo "===== C. LIST + GET ====="
ck "list.after.count"   "2"            "$(curl -s $B/datasources "${VW[@]}" | jq 'length')"
ck "get.one.label"      "Cloud Warehouse" "$(curl -s $B/datasources/warehouse "${VW[@]}" | jq -r .label)"
ck_code "get.missing.404" 404 "$B/datasources/nope" "${VW[@]}"

echo; echo "===== D. FEDERATED QUERY (SurrealDB + live Postgres via /query) ====="
# Count a real column, not count(*): a wildcard count over a federated
# datafusion-table-providers source trips an upstream DataFusion schema bug
# (physical-vs-logical projection mismatch), unrelated to the rubix wiring.
Q='{"sql":"SELECT count(id) AS n FROM \"warehouse\".\"warehouse_readings\""}'
ck "query.federated.count" "3" "$(curl -s -X POST $B/query "${AN[@]}" "${JSON[@]}" -d "$Q" | jq '.rows[0].n')"
QSUM='{"sql":"SELECT sum(value) AS s FROM \"warehouse\".\"warehouse_readings\" WHERE site = '"'"'hq'"'"'"}'
ck "query.federated.sum"   "22" "$(curl -s -X POST $B/query "${AN[@]}" "${JSON[@]}" -d "$QSUM" | jq '.rows[0].s | floor')"
# native records still queryable on the same surface
ck "query.native.still"    "660" "$(curl -s -X POST $B/query "${AN[@]}" "${JSON[@]}" -d '{"sql":"SELECT count(*) AS n FROM record"}' | jq '.rows[0].n')"

echo; echo "===== E. UPDATE (PATCH) ====="
ck "update.label" "Renamed WH" "$(curl -s -X PATCH $B/datasources/warehouse "${OP[@]}" "${JSON[@]}" -d '{"label":"Renamed WH"}' | jq -r .label)"
ck "update.applied" "Renamed WH" "$(curl -s $B/datasources/warehouse "${VW[@]}" | jq -r .label)"
ck_code "update.viewer.403" 403 -X PATCH "$B/datasources/warehouse" "${VW[@]}" "${JSON[@]}" -d '{"label":"x"}'

echo; echo "===== F. PERSISTENCE ACROSS RESTART (rehydrate) ====="
echo "### restarting server WITHOUT --seed-dev (same data dir) ..."
boot || exit 1
ck "rehydrate.present"  "warehouse"  "$(curl -s $B/datasources/warehouse "${VW[@]}" | jq -r .id)"
ck "rehydrate.label"    "Renamed WH" "$(curl -s $B/datasources/warehouse "${VW[@]}" | jq -r .label)"
ck "rehydrate.query"    "3"          "$(curl -s -X POST $B/query "${AN[@]}" "${JSON[@]}" -d "$Q" | jq '.rows[0].n')"

echo; echo "===== G. DELETE ====="
ck_code "delete.viewer.403"   403 -X DELETE "$B/datasources/warehouse" "${VW[@]}"
ck_code "delete.native.403"   403 -X DELETE "$B/datasources/surrealdb" "${OP[@]}"
DEL=$(curl -s -o /dev/null -w '%{http_code}' -X DELETE "$B/datasources/warehouse" "${OP[@]}")
ck "delete.operator.204" "204" "$DEL"
ck_code "delete.gone.404"     404 "$B/datasources/warehouse" "${VW[@]}"
ck "list.back.to.native" "1"  "$(curl -s $B/datasources "${VW[@]}" | jq 'length')"

echo; echo "===== H. DELETE PERSISTS ACROSS RESTART ====="
boot || exit 1
ck "delete.stays.gone" "1" "$(curl -s $B/datasources "${VW[@]}" | jq 'length')"

echo
echo "================= RESULT: $PASS passed, $FAIL failed ================="
pkill -9 -f 'rubix-server' 2>/dev/null
[ "$FAIL" -eq 0 ]
