# rubix-query

The DataFusion query/compute surface for the rubix platform — unification and heavy compute above SurrealDB.

## What it provides

Three surfaces, all reading through a gate-issued scoped session so row-level permissions decide visibility:

- **Read-only SQL** (`run`, `run_authorized`, `ensure_read_only`) over the canonical tables — only `SELECT`/`WITH`, gated by a capability. `build_context` / `CanonicalTable` wire the DataFusion context.
- **Vectorized time-window aggregation** (`rollup_window`, `Grain`, `BucketRollup`, `Sample`) — `avg/min/max/sum/count/first/last` over `minute…week` epoch-aligned buckets. Feeds rule decisions.
- **Vector / semantic search** (`nearest`, `Neighbour`) over SurrealDB vector columns.

## Where it sits

DataFusion sits **above** SurrealDB only where cross-datasource unification or heavy vectorized aggregation is wanted; SurrealQL stays first. `rubix-datasource` registers additional `TableProvider`s into this surface; `rubix-rules` consumes its window math.

Authority: `rubix/docs/SCOPE.md` ("DataFusion — query and compute"); contracts #1, #2, #6 in `rubix/STACK-DEISGN.md`.
