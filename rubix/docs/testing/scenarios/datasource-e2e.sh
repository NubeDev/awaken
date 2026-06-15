#!/usr/bin/env bash
# End-to-end test of the Postgres/Timescale datasource connector against a live DB.
#
# Brings up the TimescaleDB container, seeds demo telemetry, then runs the
# feature-gated connector tests (connect + federated span) against it. The tests
# skip cleanly when RUBIX_TEST_PG is unset, so this script is the way to actually
# exercise them.
#
# Usage (from anywhere):
#   docs/testing/scenarios/datasource-e2e.sh           # port 5433, leaves DB up
#   DB_PORT=5444 docs/testing/scenarios/datasource-e2e.sh
#   KEEP=0 docs/testing/scenarios/datasource-e2e.sh    # tear the container down after
#
# Requires: docker.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RUBIX_DIR="$(cd "$SCRIPT_DIR/../../.." && pwd)"
cd "$RUBIX_DIR"

DB_PORT="${DB_PORT:-5433}"
KEEP="${KEEP:-1}"
export DB_PORT
PGURL="postgres://rubix:rubix@127.0.0.1:${DB_PORT}/rubix?sslmode=disable"

echo "### bringing up TimescaleDB on port ${DB_PORT} ..."
make -C "$RUBIX_DIR" db-up

echo "### seeding demo data into Postgres ..."
docker exec -i rubix-timescaledb psql -U rubix -d rubix <<'SQL'
DROP TABLE IF EXISTS rubix_datasource_probe;
CREATE TABLE rubix_datasource_probe (id int primary key, note text);
INSERT INTO rubix_datasource_probe VALUES (1,'probe-a'),(2,'probe-b');

DROP TABLE IF EXISTS sensor_readings;
CREATE TABLE sensor_readings (
  ts      timestamptz      not null,
  site    text             not null,
  equip   text             not null,
  measure text             not null,
  value   double precision not null,
  unit    text             not null
);
SELECT create_hypertable('sensor_readings','ts');
INSERT INTO sensor_readings (ts, site, equip, measure, value, unit)
SELECT now() - (g || ' hours')::interval,
       (ARRAY['hq','plant'])[1 + (g % 2)],
       (ARRAY['ahu-1','elec-main','water-main'])[1 + (g % 3)],
       (ARRAY['temp','kw','flow'])[1 + (g % 3)],
       20 + (g % 10) + random(),
       (ARRAY['degC','kW','L/min'])[1 + (g % 3)]
FROM generate_series(0,71) AS g;
SELECT 'sensor_readings rows' AS t, count(*) FROM sensor_readings;
SQL

echo "### running feature-gated connector tests against ${PGURL} ..."
RUBIX_TEST_PG="$PGURL" cargo test -p rubix-datasource --features postgres \
  --test postgres_connect --test postgres_span -- --nocapture

if [ "$KEEP" = "1" ]; then
  echo "### TimescaleDB left running on ${DB_PORT} (KEEP=0 to tear down)."
else
  echo "### tearing down TimescaleDB ..."
  make -C "$RUBIX_DIR" db-down
fi
echo "### datasource e2e complete."
