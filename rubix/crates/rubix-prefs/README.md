# rubix-prefs

Per-user display preferences for the rubix platform.

## What it provides

- **`Preferences`** — a user's display choices: **units** (metric/imperial) and **datetime** formatting.
- **`units`** (`Quantity`, `UnitSystem`, `convert`) — the metric↔imperial conversion.
- **`datetime`** (`DateTimePattern`, `format`) — per-pattern timestamp rendering.
- **`apply_to` / `FieldSpec`** — the response-DTO rewrite the transport layer invokes.

## Where it sits

Applied at the response DTO layer by `rubix-server`. Values are stored canonically (metric, RFC 3339 UTC); these preferences only change how a response is **displayed**, never what is stored — so one record renders differently per user without a second copy.

Authority: `rubix/docs/SCOPE.md` ("Preferences"); `rubix-prefs` row in `rubix/STACK-DEISGN.md`. Laid out one verb per file (`rubix/docs/FILE-LAYOUT.md`).
