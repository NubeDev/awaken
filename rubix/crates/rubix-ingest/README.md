# rubix-ingest

Zenoh ingestion + in-flight pre-processing for the rubix platform.

## What it provides

- **`subscribe`** — `authorize_keyspace` consults the gate exactly once to resolve the permitted key-space (`AuthorizedKeySpace`); `open_subscription` then declares the Zenoh subscriber (`IngestSubscriber`, `ZenohEndpoint`, `Sample`) on that scope. The gate is never touched again per message, so high-rate streams stay un-taxed; an out-of-grant key-space is refused at subscribe.
- **`process`** — the in-flight pipeline: `Decimator`, `Filter`, `Enricher`, `Pipeline`. Raw high-rate streams are processed **before** persistence, never written first and queried back.
- **`persist`** — `append_sample` writes each surviving sample as a fresh, append-only record into the partition keyed by the principal's namespace (`partition_for`, `keyspace_root`, `INGEST_ROOT`). Two edges never write the same records; reconciliation is ordering + dedup, not merge.

## Where it sits

The streaming data plane (the Zenoh stream/transport plane), distinct from the internal `rubix-bus` eventing spine.

Authority: `rubix/docs/SCOPE.md` ("Ingestion and pre-processing"); contracts #2 and #5 in `rubix/STACK-DEISGN.md`. Laid out one verb per file (`rubix/docs/FILE-LAYOUT.md`).
