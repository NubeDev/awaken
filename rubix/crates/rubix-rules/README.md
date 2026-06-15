# rubix-rules

The Rhai rules / insights runtime for the rubix platform — embedded, deterministic decisions.

## What it provides

- **`engine`** — builds the Rhai engine and the `Decision` a script returns.
- **`rule`** — the rule model: `Rule` (script + input `Binding`s + `Aggregate` inputs) and the `RuleRegistry` for composition (a rule invokes another rule).
- **`evaluate`** (`RuleRuntime`, `evaluate`, `Evaluation`, `Recorded`) — orchestrates one end-to-end evaluation. The resulting decision is:
  - **recorded** back to SurrealDB through the command gate (authorized, captured, correlated, audited — never a direct write);
  - **published** as a data-change event on the in-process bus;
  - **traced** as a per-evaluation span tree.

## Where it sits

"Rhai owns the decision; DataFusion owns the data." Window math arrives pre-computed from `rubix-query`; heavy aggregation never lives in Rhai. Rules fire offline with no cloud dependency. One correlation id threads the decision, insight, event, and every span.

Authority: `rubix/docs/SCOPE.md` ("Rhai — rules and insights"); contract #3 in `rubix/STACK-DEISGN.md`.
