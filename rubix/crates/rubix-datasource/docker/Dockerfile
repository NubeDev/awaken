# TimescaleDB (Postgres + Timescale extension) for the rubix-datasource Postgres
# connector. Built and run via the repo Makefile:
#
#   make db-up      build the image, run the container, wait until it's ready
#   make db-down    stop + remove the container (named volume preserved)
#   make db-clean   stop + remove the container and delete the data volume
#
# Connection string the connector expects (matches the env baked in below):
#   postgres://rubix:rubix@127.0.0.1:5433/rubix
FROM timescale/timescaledb:latest-pg16

ENV POSTGRES_USER=rubix \
    POSTGRES_PASSWORD=rubix \
    POSTGRES_DB=rubix

EXPOSE 5432
