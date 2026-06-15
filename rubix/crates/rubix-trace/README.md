# rubix-trace

Tracing for the rubix platform — correlated spans, emitted and bounded.

## What it provides

- **`Span`** — records one step as work flows ingest → pre-process → rule → insight → sink. Its `trace_id` is the gate/ingest `CorrelationId`, so every span of one operation shares it; spans link by `parent_span_id` into a tree.
- **`emit_span`** — publishes a span onto the in-process control bus for live subscribers.
- **`persist_span`** — appends a span to the bounded, append-only `trace` table, subject to a `SampleRate` (`RUBIX_TRACE_SAMPLE`).
- **`enforce_retention`** — caps a namespace's stored spans, evicting the oldest.
- **`assemble_trace` / `SpanNode`** — reads spans back and links them into trees by trace id.
- **`define_trace_schema`** — the trace table schema.

## Where it sits

One of three cross-cutting concerns falling out of the gate + bus chokepoints (distinct from audit and undo). The deterministic "why did this fire" record.

Authority: `rubix/docs/SCOPE.md` ("Tracing"); contracts #3 and #4 in `rubix/STACK-DEISGN.md`.
